mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{ItemEdit, ItemType, Priority, Space};
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

/// 改标题：frontmatter 与文件名同步更新，旧文件消失、新文件出现，读回一致。
#[test]
fn updating_title_renames_file_and_roundtrips() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "## 背景\n\n导出太慢。",
        )
        .expect("创建条目");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "导出报表重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                body: "## 背景\n\n导出太慢。".into(),
                ..edit()
            },
        )
        .expect("更新条目");

    let old_path = vault
        .items_dir()
        .join(format!("{}--报表导出重构.md", created.id));
    let new_path = vault
        .items_dir()
        .join(format!("{}--导出报表重构.md", created.id));
    assert!(!old_path.exists(), "旧文件应已删除：{}", old_path.display());
    let raw = std::fs::read_to_string(&new_path).expect("新文件存在");
    assert!(raw.contains("title: 导出报表重构\n"), "{raw}");

    assert_eq!(vault.list_items().expect("列出条目"), vec![updated]);
}

/// 读取条目文件原文，用来核对 frontmatter 字面量。
fn raw_file(vault: &Vault, id: &str, title: &str) -> String {
    std::fs::read_to_string(vault.items_dir().join(format!("{id}--{title}.md"))).expect("条目文件")
}

/// 可选字段写进 frontmatter，读回逐字段一致。
#[test]
fn optional_fields_roundtrip_into_frontmatter() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                project: Some("内部报表平台".into()),
                priority: Some(Priority::P1),
                tags: vec!["报表".into(), "后端".into()],
                due: Some("2026-10-30".into()),
                ..edit()
            },
        )
        .expect("更新条目");

    assert_eq!(updated.project.as_deref(), Some("内部报表平台"));
    assert_eq!(updated.priority, Some(Priority::P1));
    assert_eq!(updated.tags, vec!["报表".to_string(), "后端".to_string()]);
    assert_eq!(updated.due.as_deref(), Some("2026-10-30"));

    let raw = raw_file(&vault, &created.id, "报表导出重构");
    assert!(raw.contains("project: 内部报表平台\n"), "{raw}");
    assert!(raw.contains("priority: P1\n"), "{raw}");
    assert!(raw.contains("due: 2026-10-30\n"), "{raw}");
    assert!(raw.contains("tags:\n- 报表\n- 后端\n"), "{raw}");

    assert_eq!(vault.list_items().expect("列出条目"), vec![updated]);
}

/// 清空可选字段：frontmatter 里这些键整体消失，读回为「未设置」。
#[test]
fn clearing_optional_fields_omits_their_keys() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");
    vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                project: Some("内部报表平台".into()),
                priority: Some(Priority::P0),
                tags: vec!["报表".into()],
                due: Some("2026-10-30".into()),
                ..edit()
            },
        )
        .expect("写入可选字段");

    let cleared = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                ..edit()
            },
        )
        .expect("清空可选字段");

    assert_eq!(cleared.project, None);
    assert_eq!(cleared.priority, None);
    assert!(cleared.tags.is_empty());
    assert_eq!(cleared.due, None);

    let raw = raw_file(&vault, &created.id, "报表导出重构");
    assert!(!raw.contains("project:"), "{raw}");
    assert!(!raw.contains("priority:"), "{raw}");
    assert!(!raw.contains("due:"), "{raw}");
    assert!(!raw.contains("tags:"), "{raw}");
    assert_eq!(vault.list_items().expect("列出条目"), vec![cleared]);
}

/// 正文含中文、emoji、特殊符号与 Markdown 结构时逐字符往返。
#[test]
fn body_with_emoji_and_special_characters_roundtrips() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("正文边界", ItemType::Idea, Space::Personal, "")
        .expect("创建条目");

    let body = "## 背景 🚀\n\n- 引号 \"'` 反斜杠 \\ 井号 # 冒号: 波浪 ~\n- 表格 | a | b |\n\n---\n\n> 引用：一、二、三；emoji：😀🎉👍\n\n```rust\nlet x = \"中文\"; // 注释\n```";
    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "正文边界".into(),
                item_type: ItemType::Idea,
                space: Space::Personal,
                body: body.into(),
                ..edit()
            },
        )
        .expect("更新正文");

    assert_eq!(updated.body, body);
    assert_eq!(
        vault.list_items().expect("列出条目")[0].body,
        body,
        "读回正文应逐字符一致"
    );
}

/// 编辑不改动 id / status / created，只动可编辑字段。
#[test]
fn editing_keeps_id_status_and_created() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("原想法", ItemType::Idea, Space::Personal, "")
        .expect("创建条目");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "改过的想法".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                body: "新正文".into(),
                ..edit()
            },
        )
        .expect("更新条目");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.status, created.status);
    assert_eq!(updated.created, created.created);
    assert_eq!(updated.title, "改过的想法");
    assert_eq!(updated.item_type, ItemType::Requirement);
    assert_eq!(updated.space, Space::Work);
}

/// 空标题拒绝更新，磁盘上仍是原文件。
#[test]
fn updating_with_blank_title_is_rejected() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("原标题", ItemType::Idea, Space::Personal, "")
        .expect("创建条目");

    let error = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "   ".into(),
                ..edit()
            },
        )
        .unwrap_err();

    assert!(matches!(error, Error::InvalidTitle), "error = {error:?}");
    let raw = raw_file(&vault, &created.id, "原标题");
    assert!(raw.contains("title: 原标题\n"), "{raw}");
}

/// 更新不存在的 id 报「条目不存在」，不新建文件。
#[test]
fn updating_unknown_id_reports_item_not_found() {
    let (_dir, vault) = new_vault();

    let error = vault
        .update_item(
            "REQ-2026-9999",
            &ItemEdit {
                title: "无此条目".into(),
                ..edit()
            },
        )
        .unwrap_err();

    assert!(
        matches!(&error, Error::ItemNotFound(id) if id == "REQ-2026-9999"),
        "error = {error:?}"
    );
    assert!(vault.list_items().expect("列出条目").is_empty());
}

/// 旧文件（P0-01 写的，没有可选键）读出来即「未设置」，不会解析失败。
#[test]
fn reading_item_without_optional_keys_leaves_them_unset() {
    let (_dir, vault) = new_vault();
    std::fs::write(
        vault.items_dir().join("REQ-2026-0001--迁移前写下的条目.md"),
        "---\nid: REQ-2026-0001\ntype: requirement\ntitle: 迁移前写下的条目\nstatus: inbox\nspace: work\ncreated: 2026-10-14T10:00:00Z\n---\n\n## 背景\n\n旧文件。\n",
    )
    .expect("写入旧格式文件");

    let item = &vault.list_items().expect("列出条目")[0];
    assert_eq!(item.id, "REQ-2026-0001");
    assert_eq!(item.title, "迁移前写下的条目");
    assert_eq!(item.body, "## 背景\n\n旧文件。");
    assert_eq!(item.project, None);
    assert_eq!(item.priority, None);
    assert!(item.tags.is_empty());
    assert_eq!(item.due, None);
}

/// 前端传空串（而不是 null）时同样按「清空」处理。
#[test]
fn blank_strings_count_as_cleared() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item("空的可选字段", ItemType::Idea, Space::Personal, "")
        .expect("创建条目");

    let updated = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "空的可选字段".into(),
                item_type: ItemType::Idea,
                space: Space::Personal,
                project: Some("   ".into()),
                tags: vec!["".into(), "  ".into()],
                due: Some("".into()),
                ..edit()
            },
        )
        .expect("更新条目");

    assert_eq!(updated.project, None);
    assert!(updated.tags.is_empty());
    assert_eq!(updated.due, None);

    let raw = raw_file(&vault, &created.id, "空的可选字段");
    assert!(!raw.contains("project:"), "{raw}");
    assert!(!raw.contains("due:"), "{raw}");
    assert!(!raw.contains("tags:"), "{raw}");
}
