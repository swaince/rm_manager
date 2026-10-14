mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{ItemType, Space, Status};
use rm_manager_lib::vault::Vault;

/// 第一个垂直切片：核心写盘 + 读回，磁盘文件内容与内存模型一致。
#[test]
fn created_item_roundtrips_through_vault_file() {
    let (_dir, vault) = new_vault();

    let created = vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "## 背景\n\n导出太慢。",
        )
        .expect("创建条目");

    // 内存模型：字段取手写预期值
    assert_eq!(created.title, "报表导出重构");
    assert_eq!(created.item_type, ItemType::Requirement);
    assert_eq!(created.status, Status::Inbox);
    assert_eq!(created.space, Space::Work);
    assert_eq!(created.body, "## 背景\n\n导出太慢。");

    // id 形如 REQ-YYYY-0001，年份与 created 时间戳一致
    assert_eq!(created.id.len(), "REQ-2026-0001".len());
    assert!(created.id.starts_with("REQ-"), "id = {}", created.id);
    assert!(created.id.ends_with("-0001"), "id = {}", created.id);
    assert_eq!(
        &created.id[4..8],
        &created.created[0..4],
        "id = {}",
        created.id
    );

    // 磁盘文件：文件名 ID--标题.md，frontmatter 六个字段逐一核对字面量
    let path = vault
        .items_dir()
        .join(format!("{}--报表导出重构.md", created.id));
    let raw = std::fs::read_to_string(&path).expect("条目文件存在");
    assert!(raw.starts_with("---\n"), "文件应以 frontmatter 开头: {raw}");
    assert!(raw.contains(&format!("id: {}\n", created.id)), "{raw}");
    assert!(raw.contains("type: requirement\n"), "{raw}");
    assert!(raw.contains("title: 报表导出重构\n"), "{raw}");
    assert!(raw.contains("status: inbox\n"), "{raw}");
    assert!(raw.contains("space: work\n"), "{raw}");
    assert!(
        raw.contains(&format!("created: {}\n", created.created)),
        "{raw}"
    );
    assert!(raw.contains("## 背景\n\n导出太慢。"), "{raw}");

    // 读回（list 解析磁盘）与创建结果一致
    let listed = vault.list_items().expect("列出条目");
    assert_eq!(listed, vec![created]);
}

/// 六种类型的 id 前缀固定映射，同类型第一条序号为 0001。
#[test]
fn id_prefix_matches_item_type() {
    let (_dir, vault) = new_vault();

    let title = "样本";
    let cases = [
        (ItemType::Idea, "IDEA-2026-0001"),
        (ItemType::Requirement, "REQ-2026-0001"),
        (ItemType::Bug, "BUG-2026-0001"),
        (ItemType::Spike, "SPIKE-2026-0001"),
        (ItemType::Chore, "CHORE-2026-0001"),
        (ItemType::Session, "SESS-2026-0001"),
    ];

    for (item_type, expected) in cases {
        let item = vault
            .create_item(title, item_type, Space::Personal, "")
            .expect("创建条目");
        // 年份段取当前 UTC 年，其余部分与手写预期一致
        assert_eq!(item.id.len(), expected.len(), "id = {}", item.id);
        assert_eq!(
            &item.id[..expected.len() - 9],
            &expected[..expected.len() - 9]
        );
        assert_eq!(
            &item.id[expected.len() - 5..],
            &expected[expected.len() - 5..]
        );
        assert_eq!(
            &item.id[expected.len() - 9..expected.len() - 5],
            &item.created[..4]
        );
    }
}

/// 序号按类型各自自增，互不干扰。
#[test]
fn id_sequence_increments_per_type() {
    let (_dir, vault) = new_vault();

    let first_idea = vault
        .create_item("想法一", ItemType::Idea, Space::Personal, "")
        .unwrap();
    let second_idea = vault
        .create_item("想法二", ItemType::Idea, Space::Personal, "")
        .unwrap();
    let requirement = vault
        .create_item("需求一", ItemType::Requirement, Space::Work, "")
        .unwrap();
    let third_idea = vault
        .create_item("想法三", ItemType::Idea, Space::Personal, "")
        .unwrap();

    assert!(first_idea.id.ends_with("-0001"), "id = {}", first_idea.id);
    assert!(second_idea.id.ends_with("-0002"), "id = {}", second_idea.id);
    assert!(third_idea.id.ends_with("-0003"), "id = {}", third_idea.id);
    assert!(requirement.id.ends_with("-0001"), "id = {}", requirement.id);
    assert!(
        requirement.id.starts_with("REQ-"),
        "id = {}",
        requirement.id
    );
}

/// 重开 vault（模拟应用重启）后条目仍在，字段与正文完整。
#[test]
fn items_survive_reopening_the_vault() {
    let (dir, vault) = new_vault();
    let created = vault
        .create_item(
            "重启后仍在",
            ItemType::Spike,
            Space::Personal,
            "## 问题\n\n队列怎么选。",
        )
        .expect("创建条目");

    let reopened = Vault::open(dir.path()).expect("重新打开 vault");
    assert_eq!(reopened.list_items().expect("列出条目"), vec![created]);
}

/// 标题里的 Windows 非法字符不落进文件名，frontmatter 里的标题保持原样。
#[test]
fn file_name_sanitizes_illegal_characters() {
    let (_dir, vault) = new_vault();
    let item = vault
        .create_item("导出: 报表/重构?", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let mut names: Vec<String> = std::fs::read_dir(vault.items_dir())
        .expect("读取 items 目录")
        .map(|entry| {
            entry
                .expect("目录项")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec![format!("{}--导出- 报表-重构-.md", item.id)]);

    assert_eq!(
        vault.list_items().expect("列出条目")[0].title,
        "导出: 报表/重构?"
    );
}

/// 标题里的 YAML 特殊字符（冒号、引号、井号）原样往返。
#[test]
fn title_with_yaml_special_characters_roundtrips() {
    let (_dir, vault) = new_vault();
    let title = "报表: \"导出\" 与 # 注释";

    let created = vault
        .create_item(title, ItemType::Idea, Space::Personal, "")
        .expect("创建条目");

    assert_eq!(created.title, title);
    assert_eq!(vault.list_items().expect("列出条目")[0].title, title);
}

/// 空标题（含纯空白）拒绝写盘，vault 里不留垃圾文件。
#[test]
fn empty_title_is_rejected() {
    let (_dir, vault) = new_vault();

    let error = vault
        .create_item("   ", ItemType::Idea, Space::Personal, "")
        .unwrap_err();

    assert!(matches!(error, Error::InvalidTitle), "error = {error:?}");
    assert!(vault.list_items().expect("列出条目").is_empty());
}
