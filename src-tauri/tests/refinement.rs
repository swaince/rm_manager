mod common;

use common::new_vault;
use rm_manager_lib::error::Error;
use rm_manager_lib::model::{
    AcceptanceCriterion, Clarification, Estimate, ItemEdit, ItemType, Space, Status,
};
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
        estimate: None,
        acceptance: Vec::new(),
        clarifications: Vec::new(),
    }
}

/// 读取条目文件原文，用来核对 frontmatter 字面量。
fn raw_file(vault: &Vault, id: &str, title: &str) -> String {
    std::fs::read_to_string(vault.items_dir().join(format!("{id}--{title}.md"))).expect("条目文件")
}

/// 建一条需求并补齐就绪检查三项（验收标准全勾、已估算、澄清已答复）。
fn refined_requirement(vault: &Vault, title: &str) -> rm_manager_lib::model::Item {
    let item = vault
        .create_item(title, ItemType::Requirement, Space::Work, "")
        .expect("创建需求");
    vault
        .update_item(
            &item.id,
            &ItemEdit {
                title: title.into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                estimate: Some(Estimate::Small),
                acceptance: vec![AcceptanceCriterion {
                    text: "导出 1000 行不超过 5 秒".into(),
                    done: true,
                }],
                ..edit()
            },
        )
        .expect("补齐细化字段")
}

/// 验收标准 / 澄清记录 / 估算写入 frontmatter：字面量核对 + 读回一致。
#[test]
fn refinement_fields_roundtrip_into_frontmatter() {
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
                estimate: Some(Estimate::Small),
                acceptance: vec![
                    AcceptanceCriterion {
                        text: "导出 1000 行不超过 5 秒".into(),
                        done: true,
                    },
                    AcceptanceCriterion {
                        text: "空表导出给出友好提示".into(),
                        done: false,
                    },
                ],
                clarifications: vec![Clarification {
                    question: "历史数据要不要一起迁移".into(),
                    answer: Some("只迁近一年".into()),
                    source: Some("产品".into()),
                }],
                ..edit()
            },
        )
        .expect("更新细化字段");

    assert_eq!(updated.estimate, Some(Estimate::Small));
    assert_eq!(updated.acceptance.len(), 2);
    assert!(updated.acceptance[0].done);
    assert_eq!(
        updated.clarifications[0].answer.as_deref(),
        Some("只迁近一年")
    );
    assert_eq!(updated.clarifications[0].source.as_deref(), Some("产品"));

    let raw = raw_file(&vault, &created.id, "报表导出重构");
    assert!(raw.contains("estimate: small\n"), "{raw}");
    assert!(
        raw.contains(
            "acceptance:\n- text: 导出 1000 行不超过 5 秒\n  done: true\n- text: 空表导出给出友好提示\n  done: false\n"
        ),
        "{raw}"
    );
    assert!(
        raw.contains(
            "clarifications:\n- question: 历史数据要不要一起迁移\n  answer: 只迁近一年\n  source: 产品\n"
        ),
        "{raw}"
    );

    assert_eq!(vault.list_items().expect("列出条目"), vec![updated]);
}

/// 未答复的澄清只记问题：answer / source 的键省略；清空三个字段后 frontmatter 里整体消失。
#[test]
fn clearing_refinement_fields_omits_their_keys() {
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
                estimate: Some(Estimate::Large),
                acceptance: vec![AcceptanceCriterion {
                    text: "导出 1000 行不超过 5 秒".into(),
                    done: false,
                }],
                clarifications: vec![
                    Clarification {
                        question: "历史数据要不要一起迁移".into(),
                        answer: Some("只迁近一年".into()),
                        source: Some("产品".into()),
                    },
                    Clarification {
                        question: "导出失败怎么重试".into(),
                        answer: None,
                        source: None,
                    },
                ],
                ..edit()
            },
        )
        .expect("写入细化字段");

    let raw = raw_file(&vault, &created.id, "报表导出重构");
    assert!(raw.contains("estimate: large\n"), "{raw}");
    assert!(
        raw.contains("clarifications:\n- question: 历史数据要不要一起迁移\n"),
        "{raw}"
    );
    // 第二条只有问题：answer / source 不落键
    assert_eq!(raw.matches("answer:").count(), 1, "{raw}");
    assert_eq!(raw.matches("source:").count(), 1, "{raw}");

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
        .expect("清空细化字段");

    assert_eq!(cleared.estimate, None);
    assert!(cleared.acceptance.is_empty());
    assert!(cleared.clarifications.is_empty());

    let raw = raw_file(&vault, &created.id, "报表导出重构");
    assert!(!raw.contains("estimate:"), "{raw}");
    assert!(!raw.contains("acceptance:"), "{raw}");
    assert!(!raw.contains("clarifications:"), "{raw}");
    assert_eq!(vault.list_items().expect("列出条目"), vec![cleared]);
}

/// 空白的验收标准 / 澄清问题不入库：文本 trim 后为空即丢弃，答复与来源人的空白按未填处理。
#[test]
fn blank_refinement_entries_are_dropped() {
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
                acceptance: vec![
                    AcceptanceCriterion {
                        text: "   ".into(),
                        done: false,
                    },
                    AcceptanceCriterion {
                        text: " 导出 1000 行不超过 5 秒 ".into(),
                        done: false,
                    },
                ],
                clarifications: vec![Clarification {
                    question: " 历史数据要不要一起迁移 ".into(),
                    answer: Some("  ".into()),
                    source: Some("  ".into()),
                }],
                ..edit()
            },
        )
        .expect("更新细化字段");

    assert_eq!(updated.acceptance.len(), 1);
    assert_eq!(updated.acceptance[0].text, "导出 1000 行不超过 5 秒");
    assert_eq!(updated.clarifications[0].question, "历史数据要不要一起迁移");
    assert_eq!(updated.clarifications[0].answer, None);
    assert_eq!(updated.clarifications[0].source, None);
}

/// 就绪检查未满足项按固定顺序列出：验收标准 → 估算 → 澄清（前端提示区直接用）。
#[test]
fn dor_gaps_lists_unmet_items() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建需求");
    vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                acceptance: vec![
                    AcceptanceCriterion {
                        text: "导出 1000 行不超过 5 秒".into(),
                        done: false,
                    },
                    AcceptanceCriterion {
                        text: "空表导出给出友好提示".into(),
                        done: false,
                    },
                ],
                clarifications: vec![Clarification {
                    question: "历史数据要不要一起迁移".into(),
                    answer: None,
                    source: None,
                }],
                ..edit()
            },
        )
        .expect("写入细化字段");

    assert_eq!(
        vault.dor_gaps(&requirement.id).expect("列出未满足项"),
        vec![
            "验收标准还有 2 条未勾选".to_string(),
            "未估算".to_string(),
            "澄清问题还有 1 条待答复".to_string(),
        ]
    );

    vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                estimate: Some(Estimate::Medium),
                acceptance: vec![AcceptanceCriterion {
                    text: "导出 1000 行不超过 5 秒".into(),
                    done: true,
                }],
                ..edit()
            },
        )
        .expect("补齐");
    assert!(
        vault
            .dor_gaps(&requirement.id)
            .expect("列出未满足项")
            .is_empty()
    );
}

/// 需求缺验收标准 / 未估算 / 澄清待答复时进 ready 被拒，错误消息带明细，磁盘不动。
#[test]
fn dor_gate_rejects_requirement_with_missing_items() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建需求");

    let error = vault
        .set_status(&requirement.id, Status::Ready)
        .unwrap_err();
    let Error::InvalidStatus(message) = &error else {
        panic!("error = {error:?}");
    };
    assert!(message.contains("验收标准未填写"), "{message}");
    assert!(message.contains("未估算"), "{message}");

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: inbox\n"), "{raw}");
}

/// 验收标准未全部勾选 / 澄清未清零同样被拒。
#[test]
fn dor_gate_rejects_unmet_acceptance_and_clarifications() {
    let (_dir, vault) = new_vault();
    let requirement = refined_requirement(&vault, "报表导出重构");
    vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                estimate: Some(Estimate::Small),
                acceptance: vec![
                    AcceptanceCriterion {
                        text: "导出 1000 行不超过 5 秒".into(),
                        done: true,
                    },
                    AcceptanceCriterion {
                        text: "空表导出给出友好提示".into(),
                        done: false,
                    },
                ],
                ..edit()
            },
        )
        .expect("留一条未勾选");

    let error = vault
        .set_status(&requirement.id, Status::Ready)
        .unwrap_err();
    let Error::InvalidStatus(message) = &error else {
        panic!("error = {error:?}");
    };
    assert!(message.contains("验收标准还有 1 条未勾选"), "{message}");

    vault
        .update_item(
            &requirement.id,
            &ItemEdit {
                title: "报表导出重构".into(),
                item_type: ItemType::Requirement,
                space: Space::Work,
                estimate: Some(Estimate::Small),
                acceptance: vec![AcceptanceCriterion {
                    text: "导出 1000 行不超过 5 秒".into(),
                    done: true,
                }],
                clarifications: vec![Clarification {
                    question: "历史数据要不要一起迁移".into(),
                    answer: None,
                    source: Some("产品".into()),
                }],
                ..edit()
            },
        )
        .expect("留一条待答复澄清");

    let error = vault
        .set_status(&requirement.id, Status::Ready)
        .unwrap_err();
    let Error::InvalidStatus(message) = &error else {
        panic!("error = {error:?}");
    };
    assert!(message.contains("澄清问题还有 1 条待答复"), "{message}");
}

/// 三项补齐后进 ready 通过；非需求类型不设门（Bug / 杂务同样字段缺失也能进 ready）。
#[test]
fn dor_gate_passes_when_met_and_skips_other_types() {
    let (_dir, vault) = new_vault();
    let requirement = refined_requirement(&vault, "报表导出重构");

    let ready = vault
        .set_status(&requirement.id, Status::Ready)
        .expect("补齐后进 ready");
    assert_eq!(ready.status, Status::Ready);

    let bug = vault
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "")
        .expect("创建 Bug");
    let chore = vault
        .create_item("升级依赖", ItemType::Chore, Space::Work, "")
        .expect("创建杂务");
    for id in [&bug.id, &chore.id] {
        let moved = vault
            .set_status(id, Status::Ready)
            .expect("非需求直接进 ready");
        assert_eq!(moved.status, Status::Ready);
    }
}

/// 豁免：理由空白拒绝；填理由后放行进 ready，dor_waived 写进 frontmatter 留痕。
#[test]
fn waived_requirement_enters_ready_with_reason_recorded() {
    let (_dir, vault) = new_vault();
    let requirement = vault
        .create_item("报表导出重构", ItemType::Requirement, Space::Work, "")
        .expect("创建需求");

    let error = vault.waive_dor(&requirement.id, "   ").unwrap_err();
    assert!(
        matches!(error, Error::InvalidStatus(_)),
        "error = {error:?}"
    );
    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(!raw.contains("dor_waived"), "{raw}");

    // 未豁免时被门拦住
    assert!(vault.set_status(&requirement.id, Status::Ready).is_err());

    let waived = vault
        .waive_dor(&requirement.id, " 本季度先做，澄清在排期后补 ")
        .expect("豁免就绪检查");
    assert_eq!(
        waived.dor_waived.as_deref(),
        Some("本季度先做，澄清在排期后补")
    );

    let ready = vault
        .set_status(&requirement.id, Status::Ready)
        .expect("豁免后进 ready");
    assert_eq!(ready.status, Status::Ready);
    assert!(
        !vault
            .dor_gaps(&requirement.id)
            .expect("未满足项")
            .is_empty(),
        "豁免不改变未满足项，只是放行"
    );

    let raw = raw_file(&vault, &requirement.id, "报表导出重构");
    assert!(raw.contains("status: ready\n"), "{raw}");
    assert!(
        raw.contains("dor_waived: 本季度先做，澄清在排期后补\n"),
        "{raw}"
    );
    assert_eq!(
        vault.list_items().expect("列出条目"),
        vec![ready.clone()],
        "读回与内存一致"
    );
    assert_eq!(
        vault.list_items().expect("列出条目")[0]
            .dor_waived
            .as_deref(),
        Some("本季度先做，澄清在排期后补")
    );
}
