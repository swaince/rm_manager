mod common;

use common::new_vault;
use rm_manager_lib::index::{SavedView, SearchQuery};
use rm_manager_lib::model::{Item, ItemEdit, ItemType, Priority, Space, Status};
use rm_manager_lib::vault::Vault;

/// 测试条目规格：直接落文件——create_item / update_item 给不出非 inbox 状态与固定 created。
/// 只写测试关心的字段，其余用 `..Default::default()`。
struct Spec<'a> {
    id: &'a str,
    item_type: &'a str,
    title: &'a str,
    status: &'a str,
    space: &'a str,
    created: &'a str,
    tags: &'a [&'a str],
    project: Option<&'a str>,
    priority: Option<&'a str>,
    due: Option<&'a str>,
    body: &'a str,
}

impl Default for Spec<'_> {
    fn default() -> Self {
        Self {
            id: "REQ-2026-0001",
            item_type: "requirement",
            title: "未命名",
            status: "inbox",
            space: "work",
            created: "2026-10-14T10:00:00Z",
            tags: &[],
            project: None,
            priority: None,
            due: None,
            body: "",
        }
    }
}

/// 落一个条目文件（内容是真源；索引必须能从它推出来）。
fn put(vault: &Vault, spec: Spec) {
    let mut frontmatter = format!(
        "id: {}\ntype: {}\ntitle: {}\nstatus: {}\nspace: {}\ncreated: {}\n",
        spec.id, spec.item_type, spec.title, spec.status, spec.space, spec.created
    );
    if let Some(project) = spec.project {
        frontmatter.push_str(&format!("project: {project}\n"));
    }
    if let Some(priority) = spec.priority {
        frontmatter.push_str(&format!("priority: {priority}\n"));
    }
    if let Some(due) = spec.due {
        frontmatter.push_str(&format!("due: {due}\n"));
    }
    if !spec.tags.is_empty() {
        frontmatter.push_str("tags:\n");
        for tag in spec.tags {
            frontmatter.push_str(&format!("- {tag}\n"));
        }
    }
    std::fs::write(
        vault
            .items_dir()
            .join(format!("{}--{}.md", spec.id, spec.title)),
        format!("---\n{frontmatter}---\n\n{}\n", spec.body),
    )
    .expect("写入条目文件");
}

/// 搜索命中的条目 id，按返回顺序。
fn ids(items: &[Item]) -> Vec<String> {
    items.iter().map(|item| item.id.clone()).collect()
}

/// 索引数据库文件（删掉 / 写坏都靠它）。
fn db_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join(".rm-index").join("index.db")
}

/// 三条样本：一条中文标题含「报表导出」、一条正文含「报表引擎」、一条完全无关。
fn seed(vault: &Vault) {
    put(
        vault,
        Spec {
            id: "REQ-2026-0001",
            title: "报表导出重构",
            created: "2026-10-14T10:00:00Z",
            body: "## 背景\n\n导出太慢。",
            ..Spec::default()
        },
    );
    put(
        vault,
        Spec {
            id: "SPIKE-2026-0001",
            item_type: "spike",
            title: "消息队列选型",
            created: "2026-10-15T10:00:00Z",
            body: "## 背景\n\n对比报表引擎的吞吐。",
            ..Spec::default()
        },
    );
    put(
        vault,
        Spec {
            id: "IDEA-2026-0001",
            item_type: "idea",
            title: "爬山计划",
            space: "personal",
            created: "2026-10-16T10:00:00Z",
            body: "周末去香山。",
            ..Spec::default()
        },
    );
}

/// 全文搜索命中中文标题与正文（3 字以上走 FTS），未命中的条目不出现。
#[test]
fn full_text_search_hits_chinese_title_and_body() {
    let (_dir, vault) = new_vault();
    seed(&vault);

    // 标题命中：「报表导出」4 字
    let hits = vault
        .search(&SearchQuery {
            text: Some("报表导出".into()),
            ..SearchQuery::default()
        })
        .expect("搜索");
    assert_eq!(ids(&hits), vec!["REQ-2026-0001"]);
    assert_eq!(hits[0].title, "报表导出重构");
    assert_eq!(hits[0].body, "## 背景\n\n导出太慢。");

    // 正文命中：「报表引擎」4 字
    let hits = vault
        .search(&SearchQuery {
            text: Some("报表引擎".into()),
            ..SearchQuery::default()
        })
        .expect("搜索");
    assert_eq!(ids(&hits), vec!["SPIKE-2026-0001"]);
}

/// 2 字中文查询：trigram 不命中，回退 LIKE 子串扫描，标题与正文都算，仍按 created 降序。
#[test]
fn two_character_query_falls_back_to_substring_scan() {
    let (_dir, vault) = new_vault();
    seed(&vault);

    let hits = vault
        .search(&SearchQuery {
            text: Some("报表".into()),
            ..SearchQuery::default()
        })
        .expect("搜索");
    assert_eq!(
        ids(&hits),
        vec!["SPIKE-2026-0001", "REQ-2026-0001"],
        "正文命中 + 标题命中，按 created 降序"
    );
}

/// 空条件 = 不过滤：全部条目按 created 降序返回。
#[test]
fn empty_query_returns_everything_newest_first() {
    let (_dir, vault) = new_vault();
    seed(&vault);

    let hits = vault.search(&SearchQuery::default()).expect("搜索");
    assert_eq!(
        ids(&hits),
        vec!["IDEA-2026-0001", "SPIKE-2026-0001", "REQ-2026-0001"]
    );
}

/// 筛选组合（空间 + 状态 + 标签）跨字段 AND，任一条件不满足就不出现。
#[test]
fn filters_combine_space_status_and_tags() {
    let (_dir, vault) = new_vault();
    put(
        &vault,
        Spec {
            id: "REQ-2026-0001",
            title: "报表导出重构",
            status: "doing",
            tags: &["报表", "后端"],
            ..Spec::default()
        },
    );
    put(
        &vault,
        Spec {
            id: "REQ-2026-0002",
            title: "报表分页",
            status: "inbox",
            tags: &["报表", "后端"],
            ..Spec::default()
        },
    );
    put(
        &vault,
        Spec {
            id: "REQ-2026-0003",
            title: "个人报表",
            space: "personal",
            status: "doing",
            tags: &["报表", "后端"],
            ..Spec::default()
        },
    );
    put(
        &vault,
        Spec {
            id: "REQ-2026-0004",
            title: "后端清理",
            status: "doing",
            tags: &["后端"],
            ..Spec::default()
        },
    );

    let query = SearchQuery {
        space: Some(Space::Work),
        statuses: vec![Status::Doing],
        tags: vec!["报表".into(), "后端".into()],
        ..SearchQuery::default()
    };
    assert_eq!(
        ids(&vault.search(&query).expect("搜索")),
        vec!["REQ-2026-0001"]
    );
}

/// 类型 / 优先级 / 项目 / 日期区间筛选，与文本条件可自由组合。
#[test]
fn filters_cover_type_priority_project_and_dates() {
    let (_dir, vault) = new_vault();
    put(
        &vault,
        Spec {
            id: "REQ-2026-0001",
            title: "报表导出重构",
            project: Some("内部报表平台"),
            priority: Some("P1"),
            due: Some("2026-10-30"),
            ..Spec::default()
        },
    );
    put(
        &vault,
        Spec {
            id: "REQ-2026-0002",
            title: "报表导出重构（下季度）",
            project: Some("内部报表平台"),
            priority: Some("P2"),
            due: Some("2027-01-15"),
            ..Spec::default()
        },
    );
    put(
        &vault,
        Spec {
            id: "BUG-2026-0001",
            item_type: "bug",
            title: "报表导出乱码",
            project: Some("内部报表平台"),
            priority: Some("P1"),
            due: Some("2026-10-30"),
            ..Spec::default()
        },
    );

    let query = SearchQuery {
        text: Some("报表导出".into()),
        types: vec![ItemType::Requirement],
        priority: vec![Priority::P1],
        project: Some("内部报表平台".into()),
        due_after: Some("2026-10-01".into()),
        due_before: Some("2026-10-31".into()),
        ..SearchQuery::default()
    };
    assert_eq!(
        ids(&vault.search(&query).expect("搜索")),
        vec!["REQ-2026-0001"]
    );
}

/// 保存视图落 vault/views/<名字>.yaml；重开 vault（模拟重启）后仍能列出，查询语义等价。
#[test]
fn saved_view_survives_reopening_the_vault() {
    let (dir, vault) = new_vault();
    seed(&vault);

    let query = SearchQuery {
        space: Some(Space::Work),
        tags: vec!["报表".into()],
        ..SearchQuery::default()
    };
    let before = ids(&vault.search(&query).expect("搜索"));
    vault.save_view("进行中的报表", &query).expect("保存视图");

    let path = dir.path().join("views").join("进行中的报表.yaml");
    assert!(path.is_file(), "视图文件应存在：{}", path.display());

    // 重启：重开 vault 后视图还在，且同一条件查出同一批条目
    let reopened = Vault::open(dir.path()).expect("重新打开 vault");
    let views = reopened.list_views().expect("列出视图");
    assert_eq!(
        views,
        vec![SavedView {
            name: "进行中的报表".into(),
            query: query.clone(),
        }]
    );
    assert_eq!(
        ids(&reopened.search(&views[0].query).expect("搜索")),
        before
    );

    // 重名覆盖：同名再保存只留一条，条件换新
    let other = SearchQuery {
        text: Some("报表引擎".into()),
        ..SearchQuery::default()
    };
    reopened
        .save_view("进行中的报表", &other)
        .expect("覆盖视图");
    let views = reopened.list_views().expect("列出视图");
    assert_eq!(views.len(), 1, "同名视图只留一条：{views:?}");
    assert_eq!(views[0].query, other);

    // 删除：文件消失，再删同名报「视图不存在」
    reopened.delete_view("进行中的报表").expect("删除视图");
    assert!(reopened.list_views().expect("列出视图").is_empty());
    assert!(!path.exists(), "视图文件应已删除");
}

/// 手动重建：删掉索引文件后 rebuild_index 全量重建，结果与重建前一致。
#[test]
fn manual_rebuild_restores_identical_results() {
    let (dir, vault) = new_vault();
    seed(&vault);

    let query = SearchQuery {
        text: Some("报表".into()),
        ..SearchQuery::default()
    };
    let before = ids(&vault.search(&query).expect("搜索"));
    assert_eq!(before.len(), 2);

    std::fs::remove_file(db_path(&dir)).expect("删掉索引文件");
    vault.rebuild_index().expect("全量重建");

    assert_eq!(ids(&vault.search(&query).expect("搜索")), before);
}

/// 删掉索引文件：下次 search 自动重建，结果与删除前一致。
#[test]
fn search_rebuilds_a_deleted_index_automatically() {
    let (dir, vault) = new_vault();
    seed(&vault);

    let query = SearchQuery {
        text: Some("报表".into()),
        ..SearchQuery::default()
    };
    let before = ids(&vault.search(&query).expect("搜索"));

    std::fs::remove_file(db_path(&dir)).expect("删掉索引文件");
    vault.ensure_index().expect("校验索引");

    assert_eq!(ids(&vault.search(&query).expect("搜索")), before);
}

/// 索引文件被写入垃圾字节：自动重建，结果与损坏前一致。
#[test]
fn search_rebuilds_a_corrupted_index_automatically() {
    let (dir, vault) = new_vault();
    seed(&vault);

    let query = SearchQuery {
        text: Some("报表".into()),
        ..SearchQuery::default()
    };
    let before = ids(&vault.search(&query).expect("搜索"));

    std::fs::write(db_path(&dir), b"not a sqlite database").expect("写坏索引文件");

    assert_eq!(ids(&vault.search(&query).expect("搜索")), before);
    let head = std::fs::read(db_path(&dir)).expect("读回索引文件");
    assert!(
        head.starts_with(b"SQLite format 3\0"),
        "损坏的库应被换成真库"
    );
}

/// 条目编辑 / 删除后索引增量更新：旧词不再命中，新词命中，删掉的条目不出现。
#[test]
fn index_follows_item_edits_and_deletions() {
    let (_dir, vault) = new_vault();
    let created = vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "## 背景\n\n导出太慢。",
        )
        .expect("创建条目");
    let query = |text: &str| SearchQuery {
        text: Some(text.into()),
        ..SearchQuery::default()
    };

    assert_eq!(
        ids(&vault.search(&query("报表导出")).expect("搜索")),
        vec![created.id.clone()]
    );

    // 改标题 + 正文（标题变化会重命名文件）：旧词不再命中，新词命中
    let edited = vault
        .update_item(
            &created.id,
            &ItemEdit {
                title: "导出流水线重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                tags: vec!["流水线".into()],
                body: "## 背景\n\n改走批处理流水线。".into(),
                ..edit()
            },
        )
        .expect("更新条目");
    assert!(vault.search(&query("报表导出")).expect("搜索").is_empty());
    assert_eq!(
        ids(&vault.search(&query("批处理流水线")).expect("搜索")),
        vec![created.id.clone()]
    );

    // 标签与正文筛选同步更新
    assert_eq!(
        ids(&vault
            .search(&SearchQuery {
                tags: vec!["流水线".into()],
                ..SearchQuery::default()
            })
            .expect("搜索")),
        vec![created.id.clone()]
    );

    // 删掉文件：索引里的行跟着消失
    std::fs::remove_file(
        vault
            .items_dir()
            .join(format!("{}--{}.md", edited.id, edited.title)),
    )
    .expect("删掉条目文件");
    assert!(
        vault
            .search(&SearchQuery::default())
            .expect("搜索")
            .is_empty()
    );
}

/// 索引不进同步：打开 vault 时把 .rm-index/ 写进根 .gitignore，已有内容保留、重复打开不重复追加。
#[test]
fn opening_vault_gitignores_the_index() {
    let dir = tempfile::tempdir().expect("创建临时目录");
    std::fs::write(dir.path().join(".gitignore"), "*.tmp\n").expect("写入已有 .gitignore");

    Vault::open(dir.path()).expect("打开 vault");
    Vault::open(dir.path()).expect("再次打开 vault");

    let raw = std::fs::read_to_string(dir.path().join(".gitignore")).expect("读取 .gitignore");
    assert!(raw.contains("*.tmp"), "{raw}");
    assert_eq!(raw.matches(".rm-index/").count(), 1, "{raw}");
}

/// 编辑载荷基线：只覆盖测试要变的字段。
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
        estimate: None,
        acceptance: Vec::new(),
        clarifications: Vec::new(),
    }
}
