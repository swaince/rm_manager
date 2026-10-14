mod common;

use common::new_vault;
use rm_manager_lib::index::SearchQuery;
use rm_manager_lib::model::{ItemEdit, ItemType, Priority, Space, Status};
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

/// 剪贴板收割：标题取正文首行，正文存全文（含换行与空行），形态固定 idea / personal / inbox。
#[test]
fn harvested_clipboard_lands_as_inbox_idea_with_first_line_title() {
    let (_dir, vault) = new_vault();
    let text = "报表导出太慢\n\n从 3 月上线后，导出 3 万行要 40 秒。\n排查方向待定。";

    let item = vault.harvest_clipboard(text).expect("收割剪贴板");

    assert_eq!(item.title, "报表导出太慢");
    assert_eq!(item.body, text, "正文应保留全文含换行");
    assert_eq!(item.item_type, ItemType::Idea);
    assert_eq!(item.space, Space::Personal);
    assert_eq!(item.status, Status::Inbox);
    assert!(
        item.id.starts_with("IDEA-") && item.id.ends_with("-0001"),
        "id = {}",
        item.id
    );

    // 磁盘 frontmatter 字面量核对
    let raw = raw_file(&vault, &item.id, "报表导出太慢");
    assert!(raw.contains("type: idea\n"), "{raw}");
    assert!(raw.contains("title: 报表导出太慢\n"), "{raw}");
    assert!(raw.contains("status: inbox\n"), "{raw}");
    assert!(raw.contains("space: personal\n"), "{raw}");
    assert!(
        raw.contains("报表导出太慢\n\n从 3 月上线后，导出 3 万行要 40 秒。\n排查方向待定。"),
        "{raw}"
    );

    assert_eq!(vault.list_items().expect("列出条目"), vec![item]);
}

/// 收割标题：首行 trim 后截 80 字符（按字符不按字节）；首行空白取首个非空行；
/// 正文全空则用「剪贴板收割」兜底——核心不允许空标题。
#[test]
fn harvest_title_truncates_at_80_chars_and_falls_back_when_blank() {
    let (_dir, vault) = new_vault();

    // 100 个字符的首行 → 标题取前 80 个字符
    let long_line: String = "想法".repeat(50);
    let long = vault
        .harvest_clipboard(&format!("{long_line}\n正文"))
        .expect("收割长首行");
    assert_eq!(long.title, "想法".repeat(40), "title = {}", long.title);

    // 首行是空白 → 取首个非空行
    let blank_first = vault
        .harvest_clipboard("   \n\n真正的首行\n正文")
        .expect("收割首行空白");
    assert_eq!(blank_first.title, "真正的首行");

    // 正文全空 → 兜底标题，正文为空
    let empty = vault.harvest_clipboard("  \n  ").expect("收割空白剪贴板");
    assert_eq!(empty.title, "剪贴板收割");
    assert_eq!(empty.body, "");
    assert_eq!(empty.status, Status::Inbox);
}

/// 迷你速记窗提交：标题可空，空标题由正文首行派生；标题与正文都空用「速记」兜底；
/// 类型与空间取表单选择（默认 idea / personal）。
#[test]
fn capture_form_derives_title_from_body_when_blank() {
    let (_dir, vault) = new_vault();

    let derived = vault
        .capture_item("", ItemType::Idea, Space::Personal, "第一条想法\n详细说明")
        .expect("速记");
    assert_eq!(derived.title, "第一条想法");
    assert_eq!(derived.body, "第一条想法\n详细说明");
    assert_eq!(derived.status, Status::Inbox);

    let typed = vault
        .capture_item("手工标题", ItemType::Bug, Space::Work, "")
        .expect("带标题速记");
    assert_eq!(typed.title, "手工标题");
    assert_eq!(typed.item_type, ItemType::Bug);
    assert_eq!(typed.space, Space::Work);

    let all_blank = vault
        .capture_item("  ", ItemType::Idea, Space::Personal, "")
        .expect("全空速记");
    assert_eq!(all_blank.title, "速记");
    assert_eq!(all_blank.body, "");
    assert_eq!(all_blank.status, Status::Inbox);
}

/// 整理流：转类型 + 补属性（update）再推进状态（set_status），frontmatter 字段落盘，
/// 条目离开收件箱视图（search 不再命中 status=inbox）。
#[test]
fn triage_updates_fields_and_leaves_inbox() {
    let (_dir, vault) = new_vault();
    let item = vault
        .capture_item("", ItemType::Idea, Space::Personal, "报表导出加个筛选条件")
        .expect("速记");
    assert_eq!(item.status, Status::Inbox);

    // 转类型并补齐属性：改类型不自动离开收件箱（inbox 对新类型仍合法）
    let updated = vault
        .update_item(
            &item.id,
            &ItemEdit {
                title: item.title.clone(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                project: Some("内部报表平台".into()),
                priority: Some(Priority::P1),
                tags: vec!["报表".into(), "后端".into()],
                body: item.body.clone(),
                ..edit()
            },
        )
        .expect("整理属性");
    assert_eq!(updated.status, Status::Inbox);

    // 推进：需求从收件箱进入澄清中
    let triaged = vault
        .set_status(&item.id, Status::Clarifying)
        .expect("推进状态");
    assert_eq!(triaged.status, Status::Clarifying);

    // 磁盘 frontmatter 字面量核对
    let raw = raw_file(&vault, &item.id, "报表导出加个筛选条件");
    assert!(raw.contains("type: requirement\n"), "{raw}");
    assert!(raw.contains("status: clarifying\n"), "{raw}");
    assert!(raw.contains("title: 报表导出加个筛选条件\n"), "{raw}");
    assert!(raw.contains("space: work\n"), "{raw}");
    assert!(raw.contains("project: 内部报表平台\n"), "{raw}");
    assert!(raw.contains("priority: P1\n"), "{raw}");
    assert!(raw.contains("tags:\n- 报表\n- 后端\n"), "{raw}");

    // 收件箱视图（status=inbox 的搜索）不再命中，澄清中视图命中
    let inbox = vault
        .search(&SearchQuery {
            statuses: vec![Status::Inbox],
            ..Default::default()
        })
        .expect("搜索收件箱");
    assert!(inbox.is_empty(), "收件箱应已清空：{inbox:?}");

    let clarifying = vault
        .search(&SearchQuery {
            statuses: vec![Status::Clarifying],
            ..Default::default()
        })
        .expect("搜索澄清中");
    assert_eq!(clarifying, vec![triaged]);
}

/// 整理转类型后旧状态对新类型不合法时（会话没有 inbox），核心回落到新类型默认状态。
#[test]
fn retyping_to_session_resets_status_to_active() {
    let (_dir, vault) = new_vault();
    let item = vault
        .capture_item("", ItemType::Idea, Space::Personal, "把发散记下来")
        .expect("速记");

    let retyped = vault
        .update_item(
            &item.id,
            &ItemEdit {
                title: item.title.clone(),
                item_type: ItemType::Session,
                space: item.space,
                body: item.body.clone(),
                ..edit()
            },
        )
        .expect("转会话");

    assert_eq!(retyped.status, Status::Active);
    let raw = raw_file(&vault, &item.id, "把发散记下来");
    assert!(raw.contains("type: session\n"), "{raw}");
    assert!(raw.contains("status: active\n"), "{raw}");
}
