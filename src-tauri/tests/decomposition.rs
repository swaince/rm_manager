mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{ItemEdit, ItemType, Space};
use rm_manager_lib::vault::Vault;

/// 读取条目文件原文，用来核对 frontmatter 字面量。
fn raw_file(vault: &Vault, id: &str, title: &str) -> String {
    std::fs::read_to_string(vault.items_dir().join(format!("{id}--{title}.md"))).expect("条目文件")
}

/// 设父：子条目 frontmatter 出现 `parent: 父id`，父条目自身不带该键。
#[test]
fn setting_parent_writes_parent_key_into_child_frontmatter() {
    let (_dir, vault) = new_vault();
    let parent = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");

    let updated = vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("设置父条目");

    assert_eq!(updated.parent.as_deref(), Some(parent.id.as_str()));
    let raw = raw_file(&vault, &child.id, "导出接口分页");
    assert!(raw.contains(&format!("parent: {}\n", parent.id)), "{raw}");

    let parent_raw = raw_file(&vault, &parent.id, "报表导出重构");
    assert!(!parent_raw.contains("parent:"), "{parent_raw}");
}

/// 清父：`parent` 键整体从 frontmatter 消失，读回为「未设置」。
#[test]
fn clearing_parent_removes_the_key() {
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

    let cleared = vault.set_parent(&child.id, None).expect("清空父条目");

    assert_eq!(cleared.parent, None);
    let raw = raw_file(&vault, &child.id, "导出接口分页");
    assert!(!raw.contains("parent:"), "{raw}");
}

/// 父子关系 round-trip：重开 vault（模拟重启）后 parent 仍指向父条目。
#[test]
fn parent_link_survives_reopening_the_vault() {
    let (dir, vault) = new_vault();
    let parent = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");
    vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("设置父条目");

    let reopened = Vault::open(dir.path()).expect("重新打开 vault");
    let child_back = reopened
        .list_items()
        .expect("列出条目")
        .into_iter()
        .find(|item| item.id == child.id)
        .expect("子条目仍在");
    assert_eq!(child_back.parent.as_deref(), Some(parent.id.as_str()));
}

/// 自引用拒绝：条目的父不能是自己，磁盘上不写 parent 键。
#[test]
fn setting_parent_to_itself_is_rejected() {
    let (_dir, vault) = new_vault();
    let item = vault
        .create_item("自引用", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let error = vault.set_parent(&item.id, Some(&item.id)).unwrap_err();

    assert!(
        matches!(error, Error::InvalidParent(_)),
        "error = {error:?}"
    );
    let raw = raw_file(&vault, &item.id, "自引用");
    assert!(!raw.contains("parent:"), "{raw}");
}

/// 父不存在拒绝：报「条目不存在」，子条目磁盘内容不变。
#[test]
fn setting_missing_parent_is_rejected() {
    let (_dir, vault) = new_vault();
    let child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");

    let error = vault
        .set_parent(&child.id, Some("REQ-2026-9999"))
        .unwrap_err();

    assert!(
        matches!(&error, Error::ItemNotFound(id) if id == "REQ-2026-9999"),
        "error = {error:?}"
    );
    let raw = raw_file(&vault, &child.id, "导出接口分页");
    assert!(!raw.contains("parent:"), "{raw}");
}

/// 成环拒绝：A 认 B 作父后，再让 B 认 A 作父必须被拒，A 的父子指向不变。
#[test]
fn setting_parent_that_would_create_a_cycle_is_rejected() {
    let (_dir, vault) = new_vault();
    let a = vault
        .create_item("甲", ItemType::Requirement, Space::Work, "")
        .expect("创建甲");
    let b = vault
        .create_item("乙", ItemType::Requirement, Space::Work, "")
        .expect("创建乙");
    vault.set_parent(&a.id, Some(&b.id)).expect("甲认乙为父");

    let error = vault.set_parent(&b.id, Some(&a.id)).unwrap_err();

    assert!(
        matches!(error, Error::InvalidParent(_)),
        "error = {error:?}"
    );
    let raw = raw_file(&vault, &b.id, "乙");
    assert!(!raw.contains("parent:"), "{raw}");
    let a_back = vault
        .list_items()
        .expect("列出条目")
        .into_iter()
        .find(|item| item.id == a.id)
        .expect("甲仍在");
    assert_eq!(a_back.parent.as_deref(), Some(b.id.as_str()));
}

/// 跨层成环同样拒绝：甲→乙→丙 后，丙再认甲作父必须被拒。
#[test]
fn setting_parent_that_would_create_a_deep_cycle_is_rejected() {
    let (_dir, vault) = new_vault();
    let a = vault
        .create_item("甲", ItemType::Requirement, Space::Work, "")
        .expect("创建甲");
    let b = vault
        .create_item("乙", ItemType::Requirement, Space::Work, "")
        .expect("创建乙");
    let c = vault
        .create_item("丙", ItemType::Requirement, Space::Work, "")
        .expect("创建丙");
    vault.set_parent(&c.id, Some(&b.id)).expect("丙认乙为父");
    vault.set_parent(&b.id, Some(&a.id)).expect("乙认甲为父");

    let error = vault.set_parent(&a.id, Some(&c.id)).unwrap_err();

    assert!(
        matches!(error, Error::InvalidParent(_)),
        "error = {error:?}"
    );
}

/// 层级深度只建议不强制：三层链条（甲→乙→丙）照常写入，不报错。
#[test]
fn deep_nesting_beyond_two_levels_is_allowed() {
    let (_dir, vault) = new_vault();
    let a = vault
        .create_item("甲", ItemType::Requirement, Space::Work, "")
        .expect("创建甲");
    let b = vault
        .create_item("乙", ItemType::Requirement, Space::Work, "")
        .expect("创建乙");
    let c = vault
        .create_item("丙", ItemType::Requirement, Space::Work, "")
        .expect("创建丙");

    vault.set_parent(&b.id, Some(&a.id)).expect("乙认甲为父");
    let nested = vault.set_parent(&c.id, Some(&b.id)).expect("丙认乙为父");

    assert_eq!(nested.parent.as_deref(), Some(b.id.as_str()));
}

/// children_of：只列直接子条目，按 created（同一秒创建时按 id）升序，其他条目不受影响。
#[test]
fn children_of_lists_direct_children_in_creation_order() {
    let (_dir, vault) = new_vault();
    let parent = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let first = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目一");
    let second = vault
        .create_item("导出进度条", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目二");
    let unrelated = vault
        .create_item("无关条目", ItemType::Requirement, Space::Work, "")
        .expect("创建无关条目");
    // 故意先挂 second 再挂 first：顺序应来自 created / id，而不是挂载先后
    vault
        .set_parent(&second.id, Some(&parent.id))
        .expect("挂第二个子条目");
    vault
        .set_parent(&first.id, Some(&parent.id))
        .expect("挂第一个子条目");

    let children = vault.children_of(&parent.id).expect("列出子条目");
    assert_eq!(
        children
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec![first.id.as_str(), second.id.as_str()]
    );
    assert!(
        !children.iter().any(|item| item.id == unrelated.id),
        "无关条目不应出现在子条目里"
    );

    // 无子条目的条目：空列表；无关条目自身也不受影响
    assert!(
        vault
            .children_of(&unrelated.id)
            .expect("列出子条目")
            .is_empty()
    );
    assert_eq!(vault.list_items().expect("列出条目").len(), 4);
}

/// children_of 不递归：孙辈不算直接子条目。
#[test]
fn children_of_does_not_include_grandchildren() {
    let (_dir, vault) = new_vault();
    let grandparent = vault
        .create_item("甲", ItemType::Requirement, Space::Work, "")
        .expect("创建甲");
    let parent = vault
        .create_item("乙", ItemType::Requirement, Space::Work, "")
        .expect("创建乙");
    let child = vault
        .create_item("丙", ItemType::Requirement, Space::Work, "")
        .expect("创建丙");
    vault
        .set_parent(&parent.id, Some(&grandparent.id))
        .expect("乙认甲为父");
    vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("丙认乙为父");

    let children = vault.children_of(&grandparent.id).expect("列出子条目");

    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, parent.id);
}

/// 直接把条目文件里的 status 改成目标值：状态机票合入前，用磁盘字面量模拟「子项独立推进状态」。
fn set_status_on_disk(vault: &Vault, id: &str, title: &str, status: &str) {
    let path = vault.items_dir().join(format!("{id}--{title}.md"));
    let raw = std::fs::read_to_string(&path).expect("条目文件");
    let updated = raw.replace("status: inbox\n", &format!("status: {status}\n"));
    assert!(
        updated.contains(&format!("status: {status}\n")),
        "{updated}"
    );
    std::fs::write(&path, updated).expect("写回条目文件");
}

/// 父项聚合进度：分母 = 未 dropped 的直接子项数，分子 = 其中 done 的数量（1 done / 1 doing / 1 dropped → 1/2）。
#[test]
fn parent_progress_counts_done_over_non_dropped_children() {
    let (_dir, vault) = new_vault();
    let parent = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let done_child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建已完成子项");
    let doing_child = vault
        .create_item("导出进度条", ItemType::Requirement, Space::Work, "")
        .expect("创建进行中子项");
    let dropped_child = vault
        .create_item("导出换肤", ItemType::Requirement, Space::Work, "")
        .expect("创建已放弃子项");
    for child in [&done_child, &doing_child, &dropped_child] {
        vault
            .set_parent(&child.id, Some(&parent.id))
            .expect("挂子条目");
    }
    set_status_on_disk(&vault, &done_child.id, "导出接口分页", "done");
    set_status_on_disk(&vault, &doing_child.id, "导出进度条", "doing");
    set_status_on_disk(&vault, &dropped_child.id, "导出换肤", "dropped");

    let progress = vault.parent_progress(&parent.id).expect("聚合进度");

    assert_eq!(progress.done, 1);
    assert_eq!(progress.total, 2);
}

/// 无子项 / 子项全是 dropped 时进度均为 0/0。
#[test]
fn parent_progress_is_zero_over_zero_without_counted_children() {
    let (_dir, vault) = new_vault();
    let lonely = vault
        .create_item("没有子项", ItemType::Requirement, Space::Work, "")
        .expect("创建条目");

    let progress = vault.parent_progress(&lonely.id).expect("聚合进度");
    assert_eq!(progress.done, 0);
    assert_eq!(progress.total, 0);

    let parent = vault
        .create_item("子项全放弃", ItemType::Requirement, Space::Work, "")
        .expect("创建父条目");
    let child = vault
        .create_item("放弃的子项", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");
    vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("挂子条目");
    set_status_on_disk(&vault, &child.id, "放弃的子项", "dropped");

    let progress = vault.parent_progress(&parent.id).expect("聚合进度");
    assert_eq!(progress.done, 0);
    assert_eq!(progress.total, 0);
}

/// 子项独立推进状态：改子项状态（并改标题）不动父项的任何字段与文件内容。
#[test]
fn child_progress_does_not_touch_parent_fields() {
    let (_dir, vault) = new_vault();
    let parent = vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "## 背景\n\n导出太慢。",
        )
        .expect("创建父条目");
    let child = vault
        .create_item("导出接口分页", ItemType::Requirement, Space::Work, "")
        .expect("创建子条目");
    vault
        .set_parent(&child.id, Some(&parent.id))
        .expect("挂子条目");
    let parent_raw_before = raw_file(&vault, &parent.id, "报表导出重构");
    let parent_before = vault
        .list_items()
        .expect("列出条目")
        .into_iter()
        .find(|item| item.id == parent.id)
        .expect("父条目仍在");

    set_status_on_disk(&vault, &child.id, "导出接口分页", "done");
    vault
        .update_item(
            &child.id,
            &ItemEdit {
                title: "导出接口分页（改）".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                body: String::new(),
                project: None,
                priority: None,
                tags: Vec::new(),
                due: None,
            },
        )
        .expect("编辑子条目正文");

    let parent_after = vault
        .list_items()
        .expect("列出条目")
        .into_iter()
        .find(|item| item.id == parent.id)
        .expect("父条目仍在");
    assert_eq!(parent_after, parent_before);
    assert_eq!(
        raw_file(&vault, &parent.id, "报表导出重构"),
        parent_raw_before
    );
    // 子项自己变了：状态 done，且仍挂在父项下
    assert_eq!(vault.children_of(&parent.id).expect("列出子条目").len(), 1);
}
