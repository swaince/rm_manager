mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{ItemEdit, ItemType, Space};
use rm_manager_lib::vault::Vault;

/// 编辑载荷的基线：只覆盖测试要变的字段（`ItemEdit { title: ..., ..edit() }`）。
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

/// 排期：scheduled_week 存该周周一的日期，frontmatter 写 `2026-10-12` 字面量；
/// 重开 vault（模拟重启）后从文件读回。
#[test]
fn scheduling_into_week_writes_monday_literal_and_roundtrips() {
    let (dir, vault) = new_vault();
    let created = vault
        .create_item("周排期", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let scheduled = vault
        .schedule_item(&created.id, Some("2026-10-12"))
        .expect("排入某一周");
    assert_eq!(scheduled.scheduled_week.as_deref(), Some("2026-10-12"));

    let raw = raw_file(&vault, &created.id, "周排期");
    assert!(raw.contains("scheduled_week: 2026-10-12\n"), "{raw}");

    let reopened = Vault::open(dir.path()).expect("重新打开 vault");
    assert_eq!(reopened.list_items().expect("列出条目"), vec![scheduled]);
}

/// 取消排期：frontmatter 里 scheduled_week 键消失，读回为「未设置」。
#[test]
fn clearing_scheduled_week_omits_its_key() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("周排期", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .schedule_item(&created.id, Some("2026-10-12"))
        .expect("排入某一周");

    let cleared = vault.schedule_item(&created.id, None).expect("取消排期");
    assert_eq!(cleared.scheduled_week, None);

    let raw = raw_file(&vault, &created.id, "周排期");
    assert!(!raw.contains("scheduled_week:"), "{raw}");
}

/// 今日焦点：focus_on 写 `2026-10-14` 字面量；重开 vault 后仍然在读回结果里。
#[test]
fn setting_focus_writes_date_literal_and_roundtrips() {
    let (dir, vault) = new_vault();
    let created = vault
        .create_item("今日焦点", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let focused = vault
        .set_focus(&created.id, Some("2026-10-14"))
        .expect("选入今日焦点");
    assert_eq!(focused.focus_on.as_deref(), Some("2026-10-14"));

    let raw = raw_file(&vault, &created.id, "今日焦点");
    assert!(raw.contains("focus_on: 2026-10-14\n"), "{raw}");

    let reopened = Vault::open(dir.path()).expect("重新打开 vault");
    assert_eq!(reopened.list_items().expect("列出条目"), vec![focused]);
}

/// 取消今日焦点：frontmatter 里 focus_on 键消失，读回为「未设置」。
#[test]
fn clearing_focus_omits_its_key() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("今日焦点", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .set_focus(&created.id, Some("2026-10-14"))
        .expect("选入今日焦点");

    let cleared = vault.set_focus(&created.id, None).expect("取消今日焦点");
    assert_eq!(cleared.focus_on, None);

    let raw = raw_file(&vault, &created.id, "今日焦点");
    assert!(!raw.contains("focus_on:"), "{raw}");
}

/// 排期与焦点不参与编辑载荷：update_item 改标题 / 正文 / 目标日期后两字段保留。
#[test]
fn updating_item_keeps_scheduled_week_and_focus() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("先排期再编辑", ItemType::Requirement, Space::Work, "旧正文")
        .expect("创建条目");
    vault
        .schedule_item(&created.id, Some("2026-10-12"))
        .expect("排入某一周");
    vault
        .set_focus(&created.id, Some("2026-10-14"))
        .expect("选入今日焦点");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "改标题后排期不变".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                body: "新正文".into(),
                due: Some("2026-10-30".into()),
                ..edit()
            },
        )
        .expect("更新条目");

    assert_eq!(updated.scheduled_week.as_deref(), Some("2026-10-12"));
    assert_eq!(updated.focus_on.as_deref(), Some("2026-10-14"));

    let raw = raw_file(&vault, &created.id, "改标题后排期不变");
    assert!(raw.contains("scheduled_week: 2026-10-12\n"), "{raw}");
    assert!(raw.contains("focus_on: 2026-10-14\n"), "{raw}");
}

/// 对不存在的 id 排期 / 设焦点都报「条目不存在」，不新建文件。
#[test]
fn planning_unknown_id_reports_item_not_found() {
    let (_dir, vault) = new_vault();

    let error = vault
        .schedule_item("REQ-2026-9999", Some("2026-10-12"))
        .unwrap_err();
    assert!(
        matches!(&error, Error::ItemNotFound(id) if id == "REQ-2026-9999"),
        "error = {error:?}"
    );

    let error = vault
        .set_focus("REQ-2026-9999", Some("2026-10-14"))
        .unwrap_err();
    assert!(
        matches!(&error, Error::ItemNotFound(id) if id == "REQ-2026-9999"),
        "error = {error:?}"
    );
    assert!(vault.list_items().expect("列出条目").is_empty());
}

/// 决策：日期串不做格式校验（与 due 一致），非法串原样存储、原样读回。
/// 合法性由调用方（前端算出的周一日期）保证。
#[test]
fn malformed_week_string_is_stored_verbatim() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("非法日期串", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let scheduled = vault
        .schedule_item(&created.id, Some("第42周"))
        .expect("非法串也按普通文本存储");
    assert_eq!(scheduled.scheduled_week.as_deref(), Some("第42周"));

    let raw = raw_file(&vault, &created.id, "非法日期串");
    assert!(raw.contains("scheduled_week: 第42周\n"), "{raw}");
    assert_eq!(vault.list_items().expect("列出条目"), vec![scheduled]);
}

/// 空白串按「清空」处理（与 due 一致）。
#[test]
fn blank_week_and_focus_count_as_cleared() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("空白清空", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let cleared = vault
        .schedule_item(&created.id, Some("   "))
        .expect("空白串清空排期");
    assert_eq!(cleared.scheduled_week, None);

    let cleared = vault
        .set_focus(&created.id, Some(""))
        .expect("空串清空焦点");
    assert_eq!(cleared.focus_on, None);

    let raw = raw_file(&vault, &created.id, "空白清空");
    assert!(!raw.contains("scheduled_week:"), "{raw}");
    assert!(!raw.contains("focus_on:"), "{raw}");
}

/// 旧文件（P0-01/P0-05 写的，没有排期与焦点键）读出来即「未设置」。
#[test]
fn reading_item_without_planning_keys_leaves_them_unset() {
    let (_dir, vault) = new_vault();
    std::fs::write(
        vault.items_dir().join("REQ-2026-0001--排期前写下的条目.md"),
        "---\nid: REQ-2026-0001\ntype: requirement\ntitle: 排期前写下的条目\nstatus: ready\nspace: work\ncreated: 2026-10-14T10:00:00Z\n---\n\n旧文件。\n",
    )
    .expect("写入旧格式文件");

    let item = &vault.list_items().expect("列出条目")[0];
    assert_eq!(item.scheduled_week, None);
    assert_eq!(item.focus_on, None);
}
