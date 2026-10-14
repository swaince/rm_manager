mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{ItemEdit, ItemType, Space, Status, allowed_statuses};
use rm_manager_lib::vault::Vault;

/// 编辑载荷基线：只覆盖测试要变的字段（`ItemEdit { title: ..., ..edit() }`）。
fn edit() -> ItemEdit {
    ItemEdit {
        title: String::new(),
        item_type: ItemType::Idea,
        space: Space::Personal,
        body: String::new(),
        project: None,
        priority: None,
        tags: Vec::new(),
        due: None,
    }
}

/// 读取条目文件原文，用来核对 frontmatter 字面量。
fn raw_file(vault: &Vault, id: &str, title: &str) -> String {
    std::fs::read_to_string(vault.items_dir().join(format!("{id}--{title}.md"))).expect("条目文件")
}

/// 断言某类型的允许集合与 spec 完全一致（顺序无关）。
fn assert_allowed(item_type: ItemType, expected: &[Status]) {
    let actual = allowed_statuses(item_type);
    for status in expected {
        assert!(
            actual.contains(status),
            "{item_type:?} 应允许 {status:?}，实际 {actual:?}"
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "{item_type:?} 允许集合 = {actual:?}"
    );
}

/// 六类型状态集合与 FR-0.2 一致；blocked / dropped 对全部类型可用。
#[test]
fn allowed_statuses_match_spec_per_type() {
    use Status::*;

    assert_allowed(
        ItemType::Requirement,
        &[
            Inbox, Clarifying, Refining, Ready, Scheduled, Doing, Review, Done, Blocked, Dropped,
        ],
    );
    assert_allowed(
        ItemType::Bug,
        &[
            Inbox, Ready, Scheduled, Doing, Review, Done, Blocked, Dropped,
        ],
    );
    assert_allowed(
        ItemType::Chore,
        &[Inbox, Ready, Scheduled, Doing, Done, Blocked, Dropped],
    );
    assert_allowed(ItemType::Idea, &[Inbox, Kept, Blocked, Dropped]);
    assert_allowed(ItemType::Spike, &[Inbox, Doing, Done, Blocked, Dropped]);
    assert_allowed(ItemType::Session, &[Active, Concluded, Blocked, Dropped]);
}

/// 各类型创建默认状态：会话 = active，其余 = inbox（frontmatter 字面量核对）。
#[test]
fn create_item_defaults_status_per_type() {
    let (_dir, vault) = new_vault();

    let session = vault
        .create_item("头脑风暴会话", ItemType::Session, Space::Personal, "")
        .expect("创建会话");
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建需求");

    assert_eq!(session.status, Status::Active);
    assert_eq!(requirement.status, Status::Inbox);

    let raw = raw_file(&vault, &session.id, "头脑风暴会话");
    assert!(raw.contains("status: active\n"), "{raw}");
}

/// 集合外状态一律拒绝（想法进 done、会话回 inbox……），且拒绝时磁盘不动。
#[test]
fn set_status_rejects_targets_outside_type_set() {
    let (_dir, vault) = new_vault();
    let idea = vault
        .create_item("导出加个水印", ItemType::Idea, Space::Personal, "")
        .unwrap();
    let session = vault
        .create_item("头脑风暴会话", ItemType::Session, Space::Personal, "")
        .unwrap();
    let spike = vault
        .create_item("队列怎么选", ItemType::Spike, Space::Work, "")
        .unwrap();
    let chore = vault
        .create_item("升级依赖", ItemType::Chore, Space::Work, "")
        .unwrap();
    let bug = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .unwrap();

    let cases = [
        (&idea.id, Status::Done),
        (&idea.id, Status::Ready),
        (&session.id, Status::Inbox),
        (&session.id, Status::Done),
        (&spike.id, Status::Ready),
        (&spike.id, Status::Review),
        (&chore.id, Status::Review),
        (&chore.id, Status::Clarifying),
        (&bug.id, Status::Clarifying),
    ];
    for (id, status) in cases {
        let error = vault.set_status(id, status).unwrap_err();
        assert!(
            matches!(error, Error::InvalidStatus(_)),
            "{id} -> {status:?} 应被拒，error = {error:?}"
        );
    }

    let raw = raw_file(&vault, &idea.id, "导出加个水印");
    assert!(raw.contains("status: inbox\n"), "{raw}");
}

/// 集合内自由跳转（不做顺序强制）：inbox → done、done → clarifying、active → concluded、inbox → kept。
#[test]
fn set_status_allows_free_jumps_within_set() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    let session = vault
        .create_item("头脑风暴会话", ItemType::Session, Space::Personal, "")
        .unwrap();
    let idea = vault
        .create_item("导出加个水印", ItemType::Idea, Space::Personal, "")
        .unwrap();

    let jumped = vault
        .set_status(&requirement.id, Status::Done)
        .expect("inbox → done");
    assert_eq!(jumped.status, Status::Done);
    let back = vault
        .set_status(&requirement.id, Status::Clarifying)
        .expect("done → clarifying");
    assert_eq!(back.status, Status::Clarifying);

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: clarifying\n"), "{raw}");

    let concluded = vault
        .set_status(&session.id, Status::Concluded)
        .expect("active → concluded");
    assert_eq!(concluded.status, Status::Concluded);
    let kept = vault
        .set_status(&idea.id, Status::Kept)
        .expect("inbox → kept");
    assert_eq!(kept.status, Status::Kept);
}

/// blocked / dropped 只能走专门动作，不能从状态选择里绕过必填原因。
#[test]
fn set_status_rejects_bypass_targets() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();

    for target in [Status::Blocked, Status::Dropped] {
        let error = vault.set_status(&requirement.id, target).unwrap_err();
        assert!(
            matches!(error, Error::InvalidStatus(_)),
            "-> {target:?} 应被拒，error = {error:?}"
        );
    }

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: inbox\n"), "{raw}");
}

/// 从 doing 阻塞再解除：回到 doing，frontmatter 写入并清空旁路字段。
#[test]
fn block_then_unblock_restores_doing() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    vault
        .set_status(&requirement.id, Status::Doing)
        .expect("进入 doing");

    let blocked = vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");
    assert_eq!(blocked.status, Status::Blocked);
    assert_eq!(blocked.blocked_reason.as_deref(), Some("等运维开通账号"));
    assert_eq!(blocked.blocked_waiting_on.as_deref(), Some("运维组"));
    assert_eq!(blocked.blocked_from, Some(Status::Doing));

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: blocked\n"), "{raw}");
    assert!(raw.contains("blocked_reason: 等运维开通账号\n"), "{raw}");
    assert!(raw.contains("blocked_waiting_on: 运维组\n"), "{raw}");
    assert!(raw.contains("blocked_from: doing\n"), "{raw}");

    let unblocked = vault.unblock_item(&requirement.id).expect("解除阻塞");
    assert_eq!(unblocked.status, Status::Doing);
    assert_eq!(unblocked.blocked_reason, None);
    assert_eq!(unblocked.blocked_waiting_on, None);
    assert_eq!(unblocked.blocked_from, None);

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: doing\n"), "{raw}");
    assert!(!raw.contains("blocked_reason"), "{raw}");
    assert!(!raw.contains("blocked_waiting_on"), "{raw}");
    assert!(!raw.contains("blocked_from"), "{raw}");

    assert_eq!(vault.list_items().expect("列出条目"), vec![unblocked]);
}

/// 从 ready 阻塞再解除：回到 ready。
#[test]
fn block_then_unblock_restores_ready() {
    let (_dir, vault) = new_vault();
    let bug = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .unwrap();
    vault
        .set_status(&bug.id, Status::Ready)
        .expect("进入 ready");

    let blocked = vault
        .block_item(&bug.id, "缺复现环境", "测试同学")
        .expect("阻塞");
    assert_eq!(blocked.blocked_from, Some(Status::Ready));

    let unblocked = vault.unblock_item(&bug.id).expect("解除阻塞");
    assert_eq!(unblocked.status, Status::Ready);
}

/// 阻塞必须同时填原因与等待对象，缺一即拒且不落盘；写入前 trim。
#[test]
fn block_requires_reason_and_waiting_on() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();

    let cases = [
        ("", "运维组"),
        ("   ", "运维组"),
        ("等运维开通账号", ""),
        ("等运维开通账号", "  "),
    ];
    for (reason, waiting_on) in cases {
        let error = vault
            .block_item(&requirement.id, reason, waiting_on)
            .unwrap_err();
        assert!(
            matches!(error, Error::InvalidStatus(_)),
            "({reason:?}, {waiting_on:?}) 应被拒，error = {error:?}"
        );
    }
    assert_eq!(
        vault.list_items().expect("列出条目")[0].status,
        Status::Inbox
    );

    let blocked = vault
        .block_item(&requirement.id, " 等运维开通账号 ", " 运维组 ")
        .expect("阻塞");
    assert_eq!(blocked.blocked_reason.as_deref(), Some("等运维开通账号"));
    assert_eq!(blocked.blocked_waiting_on.as_deref(), Some("运维组"));
}

/// 已阻塞再阻塞、没阻塞就解除：都拒绝，且不改动已有阻塞内容。
#[test]
fn block_and_unblock_state_guards() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();

    let error = vault.unblock_item(&requirement.id).unwrap_err();
    assert!(
        matches!(error, Error::InvalidStatus(_)),
        "error = {error:?}"
    );

    vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");
    let error = vault
        .block_item(&requirement.id, "再等等", "运维组")
        .unwrap_err();
    assert!(
        matches!(error, Error::InvalidStatus(_)),
        "error = {error:?}"
    );

    let current = &vault.list_items().expect("列出条目")[0];
    assert_eq!(current.status, Status::Blocked);
    assert_eq!(current.blocked_reason.as_deref(), Some("等运维开通账号"));
}

/// 放弃必须填原因；frontmatter 记录 dropped_reason。
#[test]
fn drop_requires_reason_and_records_it() {
    let (_dir, vault) = new_vault();
    let idea = vault
        .create_item("导出加个水印", ItemType::Idea, Space::Personal, "")
        .unwrap();

    let error = vault.drop_item(&idea.id, "   ").unwrap_err();
    assert!(
        matches!(error, Error::InvalidStatus(_)),
        "error = {error:?}"
    );
    assert_eq!(
        vault.list_items().expect("列出条目")[0].status,
        Status::Inbox
    );

    let dropped = vault.drop_item(&idea.id, "需求已过时").expect("放弃");
    assert_eq!(dropped.status, Status::Dropped);
    assert_eq!(dropped.dropped_reason.as_deref(), Some("需求已过时"));

    let raw = raw_file(&vault, &idea.id, "导出加个水印");
    assert!(raw.contains("status: dropped\n"), "{raw}");
    assert!(raw.contains("dropped_reason: 需求已过时\n"), "{raw}");
}

/// 阻塞中直接放弃：dropped_reason 写入，阻塞字段不残留。
#[test]
fn dropping_a_blocked_item_clears_block_fields() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");

    let dropped = vault
        .drop_item(&requirement.id, "不做这个报表了")
        .expect("放弃");
    assert_eq!(dropped.status, Status::Dropped);
    assert_eq!(dropped.blocked_reason, None);
    assert_eq!(dropped.blocked_waiting_on, None);
    assert_eq!(dropped.blocked_from, None);

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("dropped_reason: 不做这个报表了\n"), "{raw}");
    assert!(!raw.contains("blocked_reason"), "{raw}");
    assert!(!raw.contains("blocked_waiting_on"), "{raw}");
    assert!(!raw.contains("blocked_from"), "{raw}");
}

/// 直接改状态离开 blocked / dropped：对应旁路字段被清掉。
#[test]
fn set_status_clears_bypass_fields() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");

    let resumed = vault
        .set_status(&requirement.id, Status::Doing)
        .expect("blocked → doing");
    assert_eq!(resumed.status, Status::Doing);
    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(!raw.contains("blocked_reason"), "{raw}");
    assert!(!raw.contains("blocked_from"), "{raw}");

    vault
        .drop_item(&requirement.id, "不做这个报表了")
        .expect("放弃");
    let revived = vault
        .set_status(&requirement.id, Status::Ready)
        .expect("dropped → ready");
    assert_eq!(revived.status, Status::Ready);
    assert_eq!(revived.dropped_reason, None);
    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(!raw.contains("dropped_reason"), "{raw}");
}

/// 改类型后旧状态不属于新类型：回到新类型默认状态；属于则原样保留。
#[test]
fn type_change_resets_status_only_when_outside_new_type() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    vault
        .set_status(&requirement.id, Status::Review)
        .expect("进入 review");

    let as_idea = vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Idea,
                ..edit()
            },
        )
        .expect("改成想法");
    assert_eq!(as_idea.item_type, ItemType::Idea);
    assert_eq!(as_idea.status, Status::Inbox);

    let bug = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .unwrap();
    vault
        .set_status(&bug.id, Status::Ready)
        .expect("进入 ready");
    let as_requirement = vault
        .update_item(
            &bug.id,
            &ItemEdit {
                title: "导出按钮点不动".into(),
                item_type: ItemType::Requirement,
                ..edit()
            },
        )
        .expect("改成需求");
    assert_eq!(as_requirement.status, Status::Ready);
}

/// 编辑正文等其他字段不丢旁路字段。
#[test]
fn editing_keeps_bypass_fields() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .unwrap();
    vault
        .block_item(&requirement.id, "等运维开通账号", "运维组")
        .expect("阻塞");

    let updated = vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                body: "## 背景\n\n导出太慢。".into(),
                ..edit()
            },
        )
        .expect("更新正文");

    assert_eq!(updated.status, Status::Blocked);
    assert_eq!(updated.blocked_from, Some(Status::Inbox));
    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: blocked\n"), "{raw}");
    assert!(raw.contains("blocked_reason: 等运维开通账号\n"), "{raw}");
}

/// frontmatter 里手写未知状态值：解析失败（非法值写不进来）。
#[test]
fn unknown_status_value_fails_to_parse() {
    let (_dir, vault) = new_vault();
    std::fs::write(
        vault.items_dir().join("REQ-2026-0001--状态写错.md"),
        "---\nid: REQ-2026-0001\ntype: requirement\ntitle: 状态写错\nstatus: flying\nspace: work\ncreated: 2026-10-14T10:00:00Z\n---\n\n写坏了。\n",
    )
    .expect("写入坏文件");

    let error = vault.list_items().unwrap_err();
    assert!(matches!(error, Error::Yaml(_)), "error = {error:?}");
}
