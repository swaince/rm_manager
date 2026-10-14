mod common;

use std::process::Command;

use common::new_vault;
use rm_manager_lib::model::{ItemEdit, ItemType, Space, Status};
use rm_manager_lib::vault::Vault;

/// 在 vault 目录里跑系统 git：测试正是从「文件系统 + git 状态」这个外部可观察面断言留痕。
fn git(vault: &Vault, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(vault.root())
        .args(args)
        .output()
        .expect("运行系统 git");
    assert!(
        output.status.success(),
        "git {args:?} 失败：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// `git log` 的提交摘要，最新在前（空仓库 = 空列表）。
fn log_messages(vault: &Vault) -> Vec<String> {
    let log = git(vault, &["log", "--format=%s"]);
    log.lines().map(str::to_string).collect()
}

/// 未提交的改动；任何写盘动作之后都应为空。
fn porcelain(vault: &Vault) -> String {
    git(vault, &["status", "--porcelain"])
}

/// 编辑载荷基线：只覆盖测试要变的字段（`ItemEdit { title: ..., ..edit() }`）。
fn edit() -> ItemEdit {
    ItemEdit {
        title: String::new(),
        item_type: ItemType::Requirement,
        space: Space::Work,
        body: String::new(),
        project: None,
        priority: None,
        tags: Vec::new(),
        due: None,
        estimate: None,
        acceptance: Vec::new(),
        clarifications: Vec::new(),
    }
}

/// vault 首次打开即为 git 仓库（本地留痕的前提）。
#[test]
fn open_initializes_git_repository() {
    let (_dir, vault) = new_vault();

    assert!(
        vault.root().join(".git").is_dir(),
        "Vault::open 应把 vault 初始化为 git 仓库"
    );
}

/// 首建条目：恰一条 commit，消息 `{id}: create`，仓库无未提交残留。
#[test]
fn create_item_makes_single_commit() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    assert_eq!(
        log_messages(&vault),
        vec![format!("{}: create", created.id)]
    );
    assert_eq!(porcelain(&vault), "");
}

/// 状态变更：消息字面量 `{id}: status {from}→{to}`（小写值）。
#[test]
fn status_change_commits_message() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .set_status(&requirement.id, Status::Doing)
        .expect("进入 doing");

    assert_eq!(
        log_messages(&vault),
        vec![
            format!("{}: status inbox→doing", requirement.id),
            format!("{}: create", requirement.id),
        ]
    );
    assert_eq!(porcelain(&vault), "");
}

/// 旁路动作走同一状态消息格式：阻塞 `inbox→blocked`，解除回 `blocked→inbox`。
#[test]
fn block_and_unblock_commit_status_messages() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");
    vault.unblock_item(&requirement.id).expect("解除阻塞");

    assert_eq!(
        log_messages(&vault),
        vec![
            format!("{}: status blocked→inbox", requirement.id),
            format!("{}: status inbox→blocked", requirement.id),
            format!("{}: create", requirement.id),
        ]
    );
}

/// 编辑提交：正文改动一条 `{id}: update`；标题改名重命名文件也进同一条提交，无残留。
#[test]
fn update_commits_body_and_rename_together() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "报表导出重构（v2）".into(),
                body: "改过正文。".into(),
                ..edit()
            },
        )
        .expect("更新条目");

    assert_eq!(updated.title, "报表导出重构（v2）");
    assert_eq!(
        log_messages(&vault),
        vec![
            format!("{}: update", created.id),
            format!("{}: create", created.id),
        ]
    );
    assert_eq!(porcelain(&vault), "");

    // 最新提交里只有新文件名，且带上改后的正文：改名与内容改动同属一条提交
    let files = git(
        &vault,
        &[
            "-c",
            "core.quotepath=false",
            "ls-tree",
            "--name-only",
            "-r",
            "HEAD",
        ],
    );
    let new_name = format!("items/{}--报表导出重构（v2）.md", created.id);
    assert!(files.contains(&new_name), "{files}");
    assert!(
        !files.contains(&format!("items/{}--报表导出重构.md", created.id)),
        "{files}"
    );
    let head_file = git(&vault, &["show", format!("HEAD:{new_name}").as_str()]);
    assert!(head_file.contains("改过正文。"), "{head_file}");
}

/// 拆解提交：消息 `{id}: parent {parent_id}` / `{id}: parent none`。
#[test]
fn set_parent_commits_parent_message() {
    let (_dir, vault) = new_vault();
    let parent = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");

    vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("设置父条目");
    vault.set_parent(&child.id, None).expect("清空父条目");

    assert_eq!(
        log_messages(&vault),
        vec![
            format!("{}: parent none", child.id),
            format!("{}: parent {}", child.id, parent.id),
            format!("{}: create", child.id),
            format!("{}: create", parent.id),
        ]
    );
}

/// 每个条目各提交各的：新建第二个条目是独立的一条 commit。
#[test]
fn each_item_gets_its_own_commit() {
    let (_dir, vault) = new_vault();
    let first = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    let second = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .expect("创建条目");

    assert_eq!(
        log_messages(&vault),
        vec![
            format!("{}: create", second.id),
            format!("{}: create", first.id),
        ]
    );
}

/// 时间线来自 git 历史（最新在前）：摘要、进入的状态，以及相邻两次状态变更的停留秒数。
#[test]
fn timeline_lists_status_changes_with_dwell_seconds() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .set_status(&requirement.id, Status::Doing)
        .expect("进入 doing");
    // 真实等待 1 秒，让两次状态变更的提交时间戳可分辨
    std::thread::sleep(std::time::Duration::from_secs(1));
    vault
        .set_status(&requirement.id, Status::Done)
        .expect("进入 done");

    let timeline = vault.item_timeline(&requirement.id).expect("时间线");

    assert_eq!(timeline.len(), 3, "{timeline:?}");
    assert_eq!(
        timeline[0].summary,
        format!("{}: status doing→done", requirement.id)
    );
    assert_eq!(timeline[0].status_to, Some(Status::Done));
    assert_eq!(
        timeline[0].dwell_seconds, None,
        "最近一次状态变更无停留时长"
    );
    assert_eq!(
        timeline[1].summary,
        format!("{}: status inbox→doing", requirement.id)
    );
    assert_eq!(timeline[1].status_to, Some(Status::Doing));
    let dwell = timeline[1].dwell_seconds.expect("doing 的停留时长");
    assert!((1..=5).contains(&dwell), "dwell = {dwell}");
    assert_eq!(timeline[2].summary, format!("{}: create", requirement.id));
    assert_eq!(timeline[2].status_to, None);
    assert_eq!(timeline[2].dwell_seconds, None);

    // at 是 RFC3339 且随提交递增（同一台机器，同一时区偏移，可直接比较）
    assert!(timeline[0].at.contains('T'), "{}", timeline[0].at);
    assert!(timeline[0].at > timeline[1].at, "{timeline:?}");
}

/// 标题改名后仍能回溯到改名前的历史（时间线跨文件名延续）。
#[test]
fn timeline_follows_renamed_item() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "报表导出重构（v2）".into(),
                ..edit()
            },
        )
        .expect("改名");

    let timeline = vault.item_timeline(&created.id).expect("时间线");

    assert_eq!(
        timeline
            .iter()
            .map(|entry| entry.summary.clone())
            .collect::<Vec<_>>(),
        vec![
            format!("{}: update", created.id),
            format!("{}: create", created.id),
        ]
    );
}

/// 时间线只含该条目自己的提交：另一个条目的提交不串进来。
#[test]
fn timeline_only_contains_its_own_commits() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    let other = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .expect("创建条目");
    vault
        .set_status(&other.id, Status::Doing)
        .expect("另一个条目改状态");

    let timeline = vault.item_timeline(&requirement.id).expect("时间线");

    assert_eq!(timeline.len(), 1, "{timeline:?}");
    assert_eq!(timeline[0].summary, format!("{}: create", requirement.id));
}
