use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::Result;
use crate::model::Status;
use crate::vault::Vault;

/// 仓库级提交身份：本机可能没配全局 user.name / user.email，缺了 git commit 会直接失败。
const IDENTITY: [(&str, &str); 2] = [
    ("user.name", "rm-manager"),
    ("user.email", "rm-manager@local"),
];

/// 所有 git 子进程都从 vault 根目录发起；Windows 下加 CREATE_NO_WINDOW 防黑框闪现。
pub(crate) fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

pub(crate) fn run(root: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    git_command(root).args(args).output()
}

/// 出错时带上 git 自己的话，便于排查。
fn failed(args: &[&str], output: &std::process::Output) -> std::io::Error {
    std::io::Error::other(format!(
        "git {} 失败：{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// 首次使用即 git 仓库：没有 `.git` 就 init，并补上缺失的仓库级身份（已有配置不覆盖）。
///
/// shortcut: 依赖系统 git 二进制（见 docs/sync-architecture.md §7）；git 缺失时这里只返回错误，
/// 调用方降级为「不留痕但保存照常」，等真有非开发用户时再按文档换内嵌 gitoxide。
pub fn init_if_needed(root: &Path) -> std::io::Result<()> {
    if !root.join(".git").is_dir() {
        let output = run(root, &["init", "-q"])?;
        if !output.status.success() {
            return Err(failed(&["init", "-q"], &output));
        }
    }
    for (key, value) in IDENTITY {
        // 已有配置（仓库级或全局）就不动；没有值时 git config 会以非零退出
        let configured = run(root, &["config", key])
            .is_ok_and(|output| !String::from_utf8_lossy(&output.stdout).trim().is_empty());
        if !configured {
            let output = run(root, &["config", key, value])?;
            if !output.status.success() {
                return Err(failed(&["config", key, value], &output));
            }
        }
    }
    Ok(())
}

/// 保存动作的自动提交：`git add -A` + `git commit`；没有改动（nothing to commit）也走这条路径，
/// 调用方一律忽略失败，见 `init_if_needed` 的降级说明。
pub fn commit_all(root: &Path, message: &str) -> std::io::Result<()> {
    let add = run(root, &["add", "-A"])?;
    if !add.status.success() {
        return Err(failed(&["add", "-A"], &add));
    }
    let commit = run(root, &["commit", "-q", "-m", message])?;
    if !commit.status.success() {
        return Err(failed(&["commit", "-m", message], &commit));
    }
    Ok(())
}

/// 提交消息格式：机器可解析（时间线由它派生状态流转），一条写盘动作一条消息。
pub fn create_message(id: &str) -> String {
    format!("{id}: create")
}

pub fn update_message(id: &str) -> String {
    format!("{id}: update")
}

/// 状态变更（含 blocked / dropped 旁路）统一格式：`{id}: status {from}→{to}`。
pub fn status_message(id: &str, from: Status, to: Status) -> String {
    format!("{id}: status {}→{}", status_text(from), status_text(to))
}

/// 拆解：`{id}: parent {parent_id}` / 顶层用 `none`。
pub fn parent_message(id: &str, parent: Option<&str>) -> String {
    format!("{id}: parent {}", parent.unwrap_or("none"))
}

/// 状态的小写字面量：直接取 serde 序列化值（与 frontmatter 同一套 rules），不手写 13 个分支。
fn status_text(status: Status) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// 反向解析状态字面量（时间线用），认不出来就是别的消息。
fn parse_status(text: &str) -> Option<Status> {
    serde_json::from_value(serde_json::Value::String(text.to_string())).ok()
}

/// 状态流转时间线的一条：进入时间、自动生成的提交摘要，以及（仅状态变更提交）
/// 进入的状态与在上一个状态停留的秒数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// 提交时间，RFC3339（带本机时区偏移；前端只截前缀显示）
    pub at: String,
    /// 提交摘要，如 `REQ-2026-0001: status ready→doing`
    pub summary: String,
    /// 本条提交进入的状态；非状态变更提交为 null
    pub status_to: Option<Status>,
    /// 在 status_to 停留的秒数（到下一次状态变更的间隔）；最近一次状态变更为 null
    pub dwell_seconds: Option<i64>,
}

impl Vault {
    /// 状态流转记录：读该条目文件的 git 提交历史，最新在前。
    ///
    /// shortcut: git 不可用 / 仓库还没有提交时返回空列表——展示功能降级为「没有记录」，
    /// 需要提示用户装 git 时再区分错误类型。
    pub fn item_timeline(&self, id: &str) -> Result<Vec<TimelineEntry>> {
        let path = self.item_path(id)?;
        let file = path
            .strip_prefix(self.root())
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // --follow：标题改名会重命名文件，条目历史要跨文件名连起来
        let Ok(output) = run(
            self.root(),
            &["log", "--follow", "--format=%aI|%s", "--", file.as_str()],
        ) else {
            return Ok(Vec::new());
        };
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let mut entries = parse_log(&String::from_utf8_lossy(&output.stdout));
        fill_dwell(&mut entries);
        Ok(entries)
    }
}

/// 解析 `git log --format=%aI|%s` 的输出（最新在前）；认不出的行跳过。
fn parse_log(text: &str) -> Vec<TimelineEntry> {
    text.lines()
        .filter_map(|line| {
            let (at, summary) = line.split_once('|')?;
            Some(TimelineEntry {
                at: at.to_string(),
                summary: summary.to_string(),
                status_to: status_to(summary),
                dwell_seconds: None,
            })
        })
        .collect()
}

/// 状态变更消息 `{id}: status {from}→{to}` 里取 to 并解析成状态。
fn status_to(summary: &str) -> Option<Status> {
    let (_, rest) = summary.split_once(": status ")?;
    let (_, to) = rest.split_once('→')?;
    parse_status(to.trim())
}

/// 停留秒数 = 到下一次（时间上更晚的）状态变更的间隔；列表最新在前，故「下一条」是已处理过的更近一条。
fn fill_dwell(entries: &mut [TimelineEntry]) {
    let mut newer: Option<OffsetDateTime> = None;
    for entry in entries.iter_mut() {
        if entry.status_to.is_none() {
            continue;
        }
        let Ok(at) = OffsetDateTime::parse(&entry.at, &Rfc3339) else {
            continue;
        };
        entry.dwell_seconds = newer.map(|next| (next - at).whole_seconds());
        newer = Some(at);
    }
}
