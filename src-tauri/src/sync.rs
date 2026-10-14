use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::{Error, Result};
use crate::git;
use crate::vault::{Vault, file_name, parse_item, render};

/// 同步状态：连接了但没有远端 = NotConfigured（本地使用不受影响）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    NotConfigured,
    Synced,
    Ahead,
    Behind,
    Diverged,
    Conflict,
    Error,
}

/// 失败分类：前端据此给可操作提示（网络：稍后重试 / 认证：更新令牌 / 冲突：去解决）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncErrorKind {
    Network,
    Auth,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncFailure {
    pub kind: SyncErrorKind,
    pub message: String,
}

/// 同步状态快照：角标、设置对话框与命令返回值共用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub state: SyncState,
    /// 本地独有、待推送的提交数
    pub ahead: u32,
    /// 远端独有、待拉取的提交数
    pub behind: u32,
    /// 本次操作拉取的提交数（纯状态查询恒为 0）
    pub pulled: u32,
    /// 本次操作推送的提交数（纯状态查询恒为 0）
    pub pushed: u32,
    pub last_error: Option<SyncFailure>,
}

/// 状态广播事件名：前端角标与冲突对话框都听它（commands / 定时任务共用）。
pub const SYNC_STATUS_EVENT: &str = "sync-status";

impl SyncStatus {
    fn not_configured() -> Self {
        Self {
            state: SyncState::NotConfigured,
            ahead: 0,
            behind: 0,
            pulled: 0,
            pushed: 0,
            last_error: None,
        }
    }

    /// 本地失败（vault 读写等）的兜底状态：前端按同一 shape 展示。
    pub fn failed(message: String) -> Self {
        Self {
            state: SyncState::Error,
            ahead: 0,
            behind: 0,
            pulled: 0,
            pushed: 0,
            last_error: Some(SyncFailure {
                kind: SyncErrorKind::Network,
                message,
            }),
        }
    }
}

/// 冲突的三选一解决方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// 用远端版本覆盖本地改动
    UseRemote,
    /// 用本地版本覆盖远端改动
    UseLocal,
    /// 两条都留：本地版留原文件，远端版另起新 id 的副本条目
    KeepBoth,
}

/// 一处冲突：路径、条目信息与两侧内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictEntry {
    /// vault 相对路径（`items/….md`），`resolve_conflict` 原样传回
    pub path: String,
    pub item_id: Option<String>,
    pub title: Option<String>,
    /// 本地侧内容（rebase 中即 stage :3:）
    pub local: String,
    /// 远端侧内容（rebase 中即 stage :2:）
    pub remote: String,
}

/// 凭据只进系统凭据库；调用 git 时逐次注入，不写进任何配置文件（docs/sync-architecture.md §5）。
const KEYRING_SERVICE: &str = "rm-manager";
/// 同步前的兜底提交：正常路径下每次写盘动作已各自提交，这条只收手工编辑等残留。
const SYNC_COMMIT_MESSAGE: &str = "sync: 自动提交本地改动";
/// 全新 vault 直接连接远端时的初始提交：没有它就没有 ref 可推。
const INIT_COMMIT_MESSAGE: &str = "chore: 初始化 vault";
/// 行尾补救（T04 遗留）：强制 LF，消灭 Windows CRLF 假冲突。
const GITATTRIBUTES_LINE: &str = "* text=auto eol=lf";
/// rebase 重放轮数上限：正常几轮内结束，纯粹防御死循环。
const MAX_REBASE_ROUNDS: usize = 64;

#[derive(Clone)]
struct Credentials {
    user: String,
    token: String,
}

impl SyncFailure {
    /// 按 git 自己输出的话分类：认证类看关键词，其余归网络（消息原样带出便于排查）。
    fn from_stderr(stderr: &[u8]) -> Self {
        let message = String::from_utf8_lossy(stderr).trim().to_string();
        Self {
            kind: classify(&message),
            message,
        }
    }
}

fn classify(message: &str) -> SyncErrorKind {
    let text = message.to_lowercase();
    const AUTH_HINTS: [&str; 9] = [
        "authentication failed",
        "could not read username",
        "could not read password",
        "terminal prompts disabled",
        "invalid username or password",
        "invalid username or token",
        "bad credentials",
        "permission denied",
        "403",
    ];
    if AUTH_HINTS.iter().any(|hint| text.contains(hint)) {
        SyncErrorKind::Auth
    } else {
        SyncErrorKind::Network
    }
}

impl Vault {
    /// 连接远端：行尾补救 → 配置 origin（已有则更新）→ 存凭据 → 首次 `push -u`。
    ///
    /// 凭据存系统凭据库，调用 git 时逐次以 `-c http.extraHeader` 注入，不落任何配置文件。
    /// 首次推送失败（网络 / 认证）不返回错误：远端配置已生效，分类写进返回状态供前端提示。
    pub fn set_remote(&self, url: &str, user: &str, token: &str) -> Result<SyncStatus> {
        let _guard = sync_guard();
        let url = url.trim();
        if url.is_empty() {
            return Err(Error::Sync("远端地址不能为空".into()));
        }
        let output = git::run(self.root(), &["config", "core.autocrlf", "false"])?;
        if !output.status.success() {
            return Err(failed(&["config", "core.autocrlf", "false"], &output));
        }
        ensure_gitattributes(self.root())?;

        let args: [&str; 4] = if self.remote_url().is_some() {
            ["remote", "set-url", "origin", url]
        } else {
            ["remote", "add", "origin", url]
        };
        run_local(self.root(), &args)?;

        let credentials = if needs_auth(url) && !token.trim().is_empty() {
            store_credential(url, user, token)
                .map_err(|error| Error::Sync(format!("凭据写入系统凭据库失败：{error}")))?;
            Some(Credentials {
                user: user.to_string(),
                token: token.to_string(),
            })
        } else {
            load_credential(url)
        };

        // 全新 vault 还没有任何提交：先落一条初始提交，否则 push 没有 ref 可推
        if !has_commit(self.root()) {
            run_local(self.root(), &["add", "-A"])?;
            run_local(
                self.root(),
                &["commit", "-q", "--allow-empty", "-m", INIT_COMMIT_MESSAGE],
            )?;
        } else {
            let _ = git::commit_all(self.root(), SYNC_COMMIT_MESSAGE);
        }

        let branch = current_branch(self.root())
            .ok_or_else(|| Error::Sync("读不到当前分支，无法推送".into()))?;
        let (ahead, _) = self.ahead_behind();
        if let Err(failure) = self.push(&branch, credentials.as_ref()) {
            return Ok(self.snapshot(Some(failure)));
        }
        let mut status = self.snapshot(None);
        status.pushed = ahead;
        Ok(status)
    }

    /// 测试连接：只读探测 `ls-remote`，不改配置、不写凭据；None = 连接成功。
    pub fn test_remote(&self, url: &str, user: &str, token: &str) -> Result<Option<SyncFailure>> {
        let url = url.trim();
        if url.is_empty() {
            return Err(Error::Sync("远端地址不能为空".into()));
        }
        let credentials = (needs_auth(url) && !token.trim().is_empty()).then(|| Credentials {
            user: user.to_string(),
            token: token.to_string(),
        });
        let output = run_remote(self.root(), &["ls-remote", url], credentials.as_ref())?;
        Ok((!output.status.success()).then(|| SyncFailure::from_stderr(&output.stderr)))
    }

    /// 同步状态（纯本地查询，不触网）。
    ///
    /// shortcut: ahead / behind 相对本地已拉取的 `origin/<branch>`——别人刚推的新提交
    /// 要等下一次 fetch（启动、定时或手动同步）才看得见。
    pub fn sync_status(&self) -> Result<SyncStatus> {
        if self.remote_url().is_none() {
            return Ok(SyncStatus::not_configured());
        }
        Ok(self.snapshot(None))
    }

    /// 同步一次：本地兜底提交 → fetch → 落后则 `pull --rebase`（autostash）→ 领先则 push。
    ///
    /// 网络 / 认证 / 冲突都不返回 Err：分类写进返回状态，失败不打断使用（FR-9.1）。
    pub fn sync(&self) -> Result<SyncStatus> {
        let _guard = sync_guard();
        // 冲突未完的现场不能碰（工作区里是带标记的文件），交回冲突解决流程
        if rebase_in_progress(self.root()) {
            return Ok(self.snapshot(None));
        }
        let _ = git::commit_all(self.root(), SYNC_COMMIT_MESSAGE);

        let Some(url) = self.remote_url() else {
            return Ok(SyncStatus::not_configured());
        };
        let credentials = load_credential(&url);
        if let Err(failure) = self.run_remote_step(&["fetch", "origin"], credentials.as_ref()) {
            return Ok(self.snapshot(Some(failure)));
        }
        let Some(branch) = current_branch(self.root()) else {
            return Ok(self.snapshot(None));
        };

        let (_, behind) = self.ahead_behind();
        if behind > 0 {
            // --autostash：工作区万一有残留改动也不中断（正常路径已提交干净）
            let args = ["pull", "--rebase", "--autostash", "origin", branch.as_str()];
            if let Err(failure) = self.run_remote_step(&args, credentials.as_ref()) {
                return Ok(self.snapshot(Some(failure)));
            }
        }

        let (ahead, _) = self.ahead_behind();
        if ahead > 0
            && let Err(failure) = self.push(&branch, credentials.as_ref())
        {
            return Ok(self.snapshot(Some(failure)));
        }
        // 快照放在推送之后：状态反映本次同步的最终结果（推完就是 Synced）
        let mut status = self.snapshot(None);
        status.pulled = behind;
        status.pushed = ahead;
        Ok(status)
    }

    /// 冲突条目列表：路径、条目 id / 标题与两侧内容，供前端逐条三选一。
    ///
    /// rebase 中 ours / theirs 语义反转：stage :2: 是 HEAD 一侧（本次同步的远端内容），
    /// :3: 才是本地改动；所以按 stage 号取，不用 `--ours` / `--theirs` 旗标。
    pub fn sync_conflicts(&self) -> Result<Vec<ConflictEntry>> {
        let mut entries = Vec::new();
        for path in unmerged_paths(self.root()) {
            let remote = stage_content(self.root(), &path, 2);
            let local = stage_content(self.root(), &path, 3);
            let parsed = parse_item(&local).or_else(|_| parse_item(&remote)).ok();
            entries.push(ConflictEntry {
                item_id: item_id_of(&path).or_else(|| parsed.as_ref().map(|item| item.id.clone())),
                title: parsed.map(|item| item.title),
                path,
                local,
                remote,
            });
        }
        Ok(entries)
    }

    /// 三选一解决一处冲突（解决后要 `git add` 标记完成，最后走 `continue_rebase_and_push`）。
    ///
    /// KeepBoth：本地版留原文件、远端版用 `next_id` 生成新 id 并以「（冲突副本 MM-DD）」
    /// 追加标题写成新条目文件——两条都活下来，任何一侧内容都不丢。
    pub fn resolve_conflict(&self, path: &str, resolution: ConflictResolution) -> Result<()> {
        let file = self.conflict_file(path)?;
        let local = stage_content(self.root(), path, 3);
        let remote = stage_content(self.root(), path, 2);
        match resolution {
            ConflictResolution::UseRemote => self.write_conflict_side(&file, path, &remote),
            ConflictResolution::UseLocal => self.write_conflict_side(&file, path, &local),
            ConflictResolution::KeepBoth => {
                self.write_conflict_side(&file, path, &local)?;
                // 远端内容解析不出条目（手工编辑损坏的极少数情况）：退化为「用本地」，
                // 远端原文仍留在 git 对象里，不写成半坏文件把整个 items/ 拖垮
                let Ok(mut item) = parse_item(&remote) else {
                    return Ok(());
                };
                let now = OffsetDateTime::now_utc();
                item.id = self.next_id(item.item_type, now.year())?;
                item.title = format!(
                    "{}（冲突副本 {:02}-{:02}）",
                    item.title,
                    now.month() as u8,
                    now.day()
                );
                let copy = self.items_dir().join(file_name(&item.id, &item.title));
                std::fs::write(&copy, render(&item)?)?;
                let relative = relative_path(self.root(), &copy);
                run_local(self.root(), &["add", "--", &relative])?;
                Ok(())
            }
        }
    }

    /// 冲突全部解决后收尾：`rebase --continue` → push（再撞冲突则回到冲突列表）。
    pub fn continue_rebase_and_push(&self) -> Result<SyncStatus> {
        let _guard = sync_guard();
        let Some(url) = self.remote_url() else {
            return Ok(SyncStatus::not_configured());
        };
        let credentials = load_credential(&url);
        let mut rounds = 0;
        while rebase_in_progress(self.root()) && rounds < MAX_REBASE_ROUNDS {
            rounds += 1;
            if !unmerged_paths(self.root()).is_empty() {
                return Ok(self.snapshot(None));
            }
            // 解决结果与远端父提交完全一致时提交会变空，git 会拦下要 --skip：
            // 语义上正是「丢弃这条本地提交」（如选了「用远端」），直接跳过
            let empty = index_matches_head(self.root());
            let args: &[&str] = if empty {
                &["rebase", "--skip"]
            } else {
                &["-c", "core.editor=true", "rebase", "--continue"]
            };
            let output = run_remote(self.root(), args, credentials.as_ref())?;
            if !output.status.success() && unmerged_paths(self.root()).is_empty() {
                return Ok(self.snapshot(Some(SyncFailure::from_stderr(&output.stderr))));
            }
        }

        let Some(branch) = current_branch(self.root()) else {
            return Ok(self.snapshot(None));
        };
        let (ahead, _) = self.ahead_behind();
        if ahead > 0
            && let Err(failure) = self.push(&branch, credentials.as_ref())
        {
            return Ok(self.snapshot(Some(failure)));
        }
        // 快照放在推送之后：状态反映本次同步的最终结果
        let mut status = self.snapshot(None);
        status.pushed = ahead;
        Ok(status)
    }

    /// 退出前推一把（FR-9.1）：带超时，超时就杀掉 git 子进程放行退出——服务端更新引用是原子的，
    /// 杀掉要么全推上要么没推上，不会留半个提交；没推上的下次启动续推。
    pub fn push_on_exit(&self, timeout: Duration) -> Result<SyncStatus> {
        let Some(_guard) = sync_guard_within(timeout) else {
            return Ok(self.snapshot(None)); // 另有同步在跑：宁可少推一次，不卡住退出
        };
        if rebase_in_progress(self.root()) {
            // 冲突未解决：不推，本地现场原样保留，下次启动继续提示解决
            return Ok(self.snapshot(None));
        }
        let Some(url) = self.remote_url() else {
            return Ok(SyncStatus::not_configured());
        };
        let Some(branch) = current_branch(self.root()) else {
            return Ok(self.snapshot(None));
        };
        let (ahead, _) = self.ahead_behind();
        if ahead == 0 {
            return Ok(self.snapshot(None));
        }
        let credentials = load_credential(&url);
        let args = ["push", "-u", "origin", branch.as_str()];
        match run_remote_with_timeout(self.root(), &args, credentials.as_ref(), timeout)? {
            Some(output) if output.status.success() => {
                let mut status = self.snapshot(None);
                status.pushed = ahead;
                Ok(status)
            }
            Some(output) => Ok(self.snapshot(Some(SyncFailure::from_stderr(&output.stderr)))),
            None => Ok(self.snapshot(Some(SyncFailure {
                kind: SyncErrorKind::Network,
                message: format!(
                    "退出推送超过 {} 秒已放弃，下次启动自动续推",
                    timeout.as_secs()
                ),
            }))),
        }
    }

    /// 一步远端操作；失败时先看仓库是否进了冲突（rebase 中止），再按 git 的输出分类。
    fn run_remote_step(
        &self,
        args: &[&str],
        credentials: Option<&Credentials>,
    ) -> std::result::Result<(), SyncFailure> {
        let output = run_remote(self.root(), args, credentials).map_err(|error| SyncFailure {
            kind: SyncErrorKind::Network,
            message: error.to_string(),
        })?;
        if output.status.success() {
            return Ok(());
        }
        let conflicts = unmerged_paths(self.root()).len();
        if conflicts > 0 {
            return Err(SyncFailure {
                kind: SyncErrorKind::Conflict,
                message: format!("{conflicts} 处冲突待解决"),
            });
        }
        Err(SyncFailure::from_stderr(&output.stderr))
    }

    /// `git push -u origin <branch>`：首次推送顺带建立跟踪关系。
    fn push(
        &self,
        branch: &str,
        credentials: Option<&Credentials>,
    ) -> std::result::Result<(), SyncFailure> {
        self.run_remote_step(&["push", "-u", "origin", branch], credentials)
    }

    fn write_conflict_side(&self, file: &Path, path: &str, content: &str) -> Result<()> {
        std::fs::write(file, content)?;
        run_local(self.root(), &["add", "--", path])?;
        Ok(())
    }

    /// 冲突路径必须落在 vault 内的相对路径上（路径来自 git，命令入参来自前端）。
    fn conflict_file(&self, path: &str) -> Result<PathBuf> {
        let candidate = Path::new(path);
        if !candidate.is_relative()
            || candidate
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(Error::Sync(format!("冲突路径不合法：{path}")));
        }
        Ok(self.root().join(candidate))
    }

    /// 状态快照：冲突中优先报冲突（带条数提示），其次按 ahead / behind 分档，最后叠加本次失败分类。
    fn snapshot(&self, last_error: Option<SyncFailure>) -> SyncStatus {
        let (ahead, behind) = self.ahead_behind();
        // rebase 停在冲突上 = 有一批待解决的冲突：重启后也能从状态里恢复出提示与分类
        let last_error = if rebase_in_progress(self.root()) {
            Some(SyncFailure {
                kind: SyncErrorKind::Conflict,
                message: format!(
                    "{} 处冲突待解决：选择保留哪一侧后再继续同步",
                    unmerged_paths(self.root()).len()
                ),
            })
        } else {
            last_error
        };
        let state = match last_error.as_ref().map(|failure| failure.kind) {
            Some(SyncErrorKind::Conflict) => SyncState::Conflict,
            Some(_) => SyncState::Error,
            None => match (ahead, behind) {
                (0, 0) => SyncState::Synced,
                (_, 0) => SyncState::Ahead,
                (0, _) => SyncState::Behind,
                _ => SyncState::Diverged,
            },
        };
        SyncStatus {
            state,
            ahead,
            behind,
            pulled: 0,
            pushed: 0,
            last_error,
        }
    }

    /// origin 的地址；未配置远端 = None。
    fn remote_url(&self) -> Option<String> {
        let output = git::run(self.root(), &["remote", "get-url", "origin"]).ok()?;
        if !output.status.success() {
            return None;
        }
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!url.is_empty()).then_some(url)
    }

    /// 与 `origin/<当前分支>` 的差距：左 = 本地独有（待推），右 = 远端独有（待拉）。
    fn ahead_behind(&self) -> (u32, u32) {
        if let Some(branch) = current_branch(self.root()) {
            let range = format!("HEAD...origin/{branch}");
            if let Some(counts) = parse_counts(
                self.root(),
                &["rev-list", "--left-right", "--count", &range],
            ) {
                return counts;
            }
        }
        // 远端还没有这个分支（连接后从未推送）：本地提交全部算待推
        let ahead = parse_counts(self.root(), &["rev-list", "--count", "HEAD"])
            .map(|(total, _)| total)
            .unwrap_or(0);
        (ahead, 0)
    }
}

/// 同步操作串行化：定时任务、手动按钮与退出钩子可能撞在一起，git 的 index.lock 不吃并发。
static SYNC_LOCK: Mutex<()> = Mutex::new(());

fn sync_guard() -> MutexGuard<'static, ()> {
    SYNC_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 拿同步锁但最多等 `wait`：退出路径专用，等不到就放弃（宁可少推一次，不卡住退出）。
fn sync_guard_within(wait: Duration) -> Option<MutexGuard<'static, ()>> {
    let deadline = Instant::now() + wait;
    loop {
        match SYNC_LOCK.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => {}
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 本地 git 命令；失败带上 git 自己的话。
fn run_local(root: &Path, args: &[&str]) -> Result<Output> {
    let output = git::run(root, args)?;
    if !output.status.success() {
        return Err(failed(args, &output));
    }
    Ok(output)
}

fn failed(args: &[&str], output: &Output) -> Error {
    Error::Sync(format!(
        "git {} 失败：{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// 远端操作统一入口：逐次注入凭据（不落任何配置文件），并禁用交互式提问——
/// GUI 没有控制台，认证失败必须立刻返回而不是挂着等输入。
fn run_remote(
    root: &Path,
    args: &[&str],
    credentials: Option<&Credentials>,
) -> std::io::Result<Output> {
    let mut command = git::git_command(root);
    if let Some(credentials) = credentials {
        command.args(auth_preamble(credentials));
    }
    command.env("GIT_TERMINAL_PROMPT", "0").args(args).output()
}

/// 带超时的远端调用：超时杀掉子进程并返回 None（退出路径专用，正常同步不设超时）。
fn run_remote_with_timeout(
    root: &Path,
    args: &[&str],
    credentials: Option<&Credentials>,
    timeout: Duration,
) -> std::io::Result<Option<Output>> {
    let mut command = git::git_command(root);
    if let Some(credentials) = credentials {
        command.args(auth_preamble(credentials));
    }
    let mut child = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout)?;
            }
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr)?;
            }
            return Ok(Some(Output {
                status,
                stdout,
                stderr,
            }));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Basic 认证串 = base64(user:token)；`-c` 只作用于这一次调用，不写进 .git/config。
fn auth_preamble(credentials: &Credentials) -> [String; 2] {
    use base64::Engine;
    let basic = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", credentials.user, credentials.token));
    [
        "-c".to_string(),
        format!("http.extraHeader=Authorization: Basic {basic}"),
    ]
}

/// 凭据只进系统凭据库（Windows 凭据管理器）；secret 是 JSON，Gitee 的 HTTPS 认证还需要用户名。
fn store_credential(url: &str, user: &str, token: &str) -> std::result::Result<(), String> {
    let secret = serde_json::json!({ "user": user, "token": token }).to_string();
    keyring::Entry::new(KEYRING_SERVICE, url)
        .and_then(|entry| entry.set_password(&secret))
        .map_err(|error| error.to_string())
}

/// 读凭据；读不到（没存过 / 凭据库不可用）就当没有：认证失败会照常按「认证」分类提示。
fn load_credential(url: &str) -> Option<Credentials> {
    if !needs_auth(url) {
        return None;
    }
    let secret = keyring::Entry::new(KEYRING_SERVICE, url)
        .ok()?
        .get_password()
        .ok()?;
    let value: serde_json::Value = serde_json::from_str(&secret).ok()?;
    Some(Credentials {
        user: value.get("user")?.as_str()?.to_string(),
        token: value.get("token")?.as_str()?.to_string(),
    })
}

/// 只有 http(s) 远端需要凭据：本地路径 / file:// 远端（测试与 U 盘兜底）没有认证。
fn needs_auth(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn has_commit(root: &Path) -> bool {
    git::run(root, &["rev-parse", "--verify", "HEAD"]).is_ok_and(|output| output.status.success())
}

fn current_branch(root: &Path) -> Option<String> {
    let output = git::run(root, &["symbolic-ref", "--short", "HEAD"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

/// 未解决的冲突文件（rebase / merge 未完成时非空）。
/// `-z` 输出原始路径：非 ASCII 文件名不会被 core.quotepath 转义成 "\\346\\212\\245" 这类串。
fn unmerged_paths(root: &Path) -> Vec<String> {
    git::run(root, &["diff", "--name-only", "--diff-filter=U", "-z"])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// 某一侧的冲突内容：rebase 中 :2: = 远端（HEAD），:3: = 本地；取不到（那一侧删除）为空串。
fn stage_content(root: &Path, path: &str, stage: u8) -> String {
    let spec = format!(":{stage}:{path}");
    git::run(root, &["show", &spec])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

fn rebase_in_progress(root: &Path) -> bool {
    ["rebase-merge", "rebase-apply"]
        .iter()
        .any(|name| git_path_exists(root, name))
}

fn git_path_exists(root: &Path, name: &str) -> bool {
    let Ok(output) = git::run(root, &["rev-parse", "--git-path", name]) else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    !path.is_empty() && root.join(path).exists()
}

/// 已解决的内容是否与父提交完全一致（rebase 中等于「提交会变空」）。
fn index_matches_head(root: &Path) -> bool {
    git::run(root, &["diff", "--cached", "--quiet"]).is_ok_and(|output| output.status.success())
}

/// `git rev-list --count` 类输出解析："a\tb" → (a, b)；"a" → (a, 0)。
fn parse_counts(root: &Path, args: &[&str]) -> Option<(u32, u32)> {
    let output = git::run(root, args).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let left = parts.next()?.parse().ok()?;
    let right = parts.next().map_or(Some(0), |value| value.parse().ok())?;
    Some((left, right))
}

/// `items/REQ-2026-0001--标题.md` → `REQ-2026-0001`。
fn item_id_of(path: &str) -> Option<String> {
    let stem = path.strip_prefix("items/")?.strip_suffix(".md")?;
    let id = stem.split("--").next().unwrap_or(stem);
    (!id.is_empty()).then(|| id.to_string())
}

/// vault 相对路径（git 认的形态：正斜杠）。
fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// 写 `.gitattributes`，缺就补、已有内容原样保留（与 vault.rs 的 .gitignore 补写同款）。
fn ensure_gitattributes(root: &Path) -> Result<()> {
    let path = root.join(".gitattributes");
    let existing = if path.is_file() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };
    if existing
        .lines()
        .any(|line| line.trim() == GITATTRIBUTES_LINE)
    {
        return Ok(());
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(GITATTRIBUTES_LINE);
    text.push('\n');
    std::fs::write(&path, text)?;
    Ok(())
}
