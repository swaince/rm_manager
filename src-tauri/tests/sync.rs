mod common;

use std::path::Path;
use std::process::Command;

use common::new_vault;
use rm_manager_lib::model::{Item, ItemEdit, ItemType, Space};
use rm_manager_lib::sync::{ConflictResolution, SyncErrorKind, SyncState};
use rm_manager_lib::vault::Vault;
use tempfile::TempDir;

/// 在指定目录里跑系统 git：测试从「文件系统 + git 状态」这个外部可观察面断言同步结果。
fn git_in(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
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

/// 未提交的改动；同步完成后必须为空（无残留）。
fn porcelain(vault: &Vault) -> String {
    git_in(vault.root(), &["status", "--porcelain"])
}

fn branch_of(vault: &Vault) -> String {
    git_in(vault.root(), &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// 本地路径形式（正斜杠）的远端地址：git 视为本地传输，无网络无鉴权（file:// 等价）。
fn path_url(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// 测试现场：裸库远端 + 已连接的 vault（两个 TempDir 必须存活到测试结束）。
struct Fixture {
    _remote_dir: TempDir,
    _vault_dir: TempDir,
    vault: Vault,
    remote: String,
}

/// A 机：新建 vault，连接裸库远端并完成首次推送。
fn connected_vault() -> Fixture {
    let remote_dir = tempfile::tempdir().expect("创建临时目录");
    let bare = remote_dir.path().join("remote.git");
    std::fs::create_dir(&bare).expect("创建裸库目录");
    git_in(&bare, &["init", "--bare", "-q"]);

    let (vault_dir, vault) = new_vault();
    vault
        .set_remote(&path_url(&bare), "", "")
        .expect("连接远端");
    // 裸库默认 HEAD 可能指向 master：指到实际推送的分支，从它 clone 才拿得到内容
    let head = format!("refs/heads/{}", branch_of(&vault));
    git_in(&bare, &["symbolic-ref", "HEAD", head.as_str()]);
    Fixture {
        _remote_dir: remote_dir,
        _vault_dir: vault_dir,
        vault,
        remote: path_url(&bare),
    }
}

/// B 机：从远端 clone 出第二个 vault（`.git` 随 clone 落地，Vault::open 直接接管）。
fn clone_vault(fixture: &Fixture) -> (TempDir, Vault) {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let target = dir.path().join("vault-b");
    let target_url = path_url(&target);
    git_in(
        dir.path(),
        &["clone", "-q", fixture.remote.as_str(), target_url.as_str()],
    );
    let vault = Vault::open(&target).expect("打开克隆出的 vault");
    (dir, vault)
}

fn item_of(vault: &Vault, id: &str) -> Item {
    vault
        .list_items()
        .expect("列出条目")
        .into_iter()
        .find(|item| item.id == id)
        .unwrap_or_else(|| panic!("条目不存在：{id}"))
}

/// 改写某条目正文（两端改同一处时制造冲突）。
fn set_body(vault: &Vault, id: &str, body: &str) {
    let item = item_of(vault, id);
    vault
        .update_item(
            id,
            &ItemEdit {
                title: item.title.clone(),
                item_type: item.item_type,
                space: item.space,
                project: None,
                priority: None,
                tags: Vec::new(),
                due: None,
                estimate: None,
                acceptance: Vec::new(),
                clarifications: Vec::new(),
                body: body.to_string(),
            },
        )
        .expect("更新条目正文");
}

/// 1. 连接私有远端：首次推送成功，裸库收到提交，ahead / behind 归零。
#[test]
fn first_push_reaches_bare_remote() {
    let fixture = connected_vault();

    let remote_log = git_in(Path::new(&fixture.remote), &["log", "--format=%s"]);
    assert!(!remote_log.is_empty(), "裸库应有首次推送的提交");

    let status = fixture.vault.sync_status().expect("同步状态");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!((status.ahead, status.behind), (0, 0));
    assert!(status.last_error.is_none());
    assert_eq!(porcelain(&fixture.vault), "");

    // T04 留下的行尾补救：仓库级 core.autocrlf=false + .gitattributes 强制 LF
    assert_eq!(
        git_in(
            fixture.vault.root(),
            &["config", "--local", "core.autocrlf"]
        ),
        "false"
    );
    let attributes = std::fs::read_to_string(fixture.vault.root().join(".gitattributes"))
        .expect(".gitattributes 应存在");
    assert!(attributes.contains("* text=auto eol=lf"), "{attributes}");
}

/// 2. 两 vault 场景：A 建条目推送 → B 从远端初始化拿到 → B 建条目推送 → A 同步拉到。
#[test]
fn two_vaults_exchange_items_through_remote() {
    let fixture = connected_vault();
    let item_a = fixture
        .vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "A 的正文",
        )
        .expect("A 建条目");
    let status = fixture.vault.sync().expect("A 同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(status.pushed, 1, "推送 A 的创建提交");
    assert_eq!(porcelain(&fixture.vault), "");

    let (_b_dir, b) = clone_vault(&fixture);
    assert!(
        b.list_items()
            .expect("B 列出条目")
            .iter()
            .any(|item| item.id == item_a.id),
        "B 应从远端拿到 A 的条目"
    );

    let item_b = b
        .create_item("导出按钮点不动", ItemType::Bug, Space::Work, "B 的正文")
        .expect("B 建条目");
    let status = b.sync().expect("B 同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(status.pushed, 1, "推送 B 的创建提交");
    assert_eq!(porcelain(&b), "");

    // A 拉 B 的条目：sync 内 fetch + pull --rebase
    let status = fixture.vault.sync().expect("A 再同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(status.pulled, 1, "A 应拉到 B 的一条提交");
    let ids: Vec<String> = fixture
        .vault
        .list_items()
        .expect("A 列出条目")
        .into_iter()
        .map(|item| item.id)
        .collect();
    assert!(
        ids.contains(&item_a.id) && ids.contains(&item_b.id),
        "{ids:?}"
    );
    assert_eq!(porcelain(&fixture.vault), "");
}

/// 冲突现场：两端改同一条目同一处、后同步的一方拉取时 rebase 中止。
struct Conflicted {
    fixture: Fixture,
    _b_dir: TempDir,
    b: Vault,
    item_id: String,
}

fn conflicting_pair() -> Conflicted {
    let fixture = connected_vault();
    let item = fixture
        .vault
        .create_item(
            "报表导出重构",
            ItemType::Requirement,
            Space::Work,
            "原始正文",
        )
        .expect("A 建条目");
    fixture.vault.sync().expect("A 推送");
    let (b_dir, b) = clone_vault(&fixture);

    set_body(&fixture.vault, &item.id, "A 改的正文");
    fixture.vault.sync().expect("A 推送改动");
    set_body(&b, &item.id, "B 改的正文");

    let status = b.sync().expect("B 同步");
    assert_eq!(status.state, SyncState::Conflict);
    Conflicted {
        fixture,
        _b_dir: b_dir,
        b,
        item_id: item.id,
    }
}

/// 3. 冲突检出：列出冲突条目，本地 / 远端内容各取对应一侧（rebase 中 stage :3: / :2:）。
#[test]
fn concurrent_edits_detected_as_conflict() {
    let pair = conflicting_pair();

    let status = pair.b.sync_status().expect("同步状态");
    assert_eq!(status.state, SyncState::Conflict);
    let failure = status.last_error.expect("冲突应带分类");
    assert_eq!(failure.kind, SyncErrorKind::Conflict);

    let conflicts = pair.b.sync_conflicts().expect("冲突列表");
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    assert_eq!(conflicts[0].item_id.as_deref(), Some(pair.item_id.as_str()));
    assert!(
        conflicts[0].local.contains("B 改的正文"),
        "{}",
        conflicts[0].local
    );
    assert!(
        conflicts[0].remote.contains("A 改的正文"),
        "{}",
        conflicts[0].remote
    );
}

/// 4a. 用远端：冲突文件取远端内容，继续 rebase + 推送后两端一致。
#[test]
fn resolve_conflict_use_remote() {
    let pair = conflicting_pair();
    let path = pair.b.sync_conflicts().expect("冲突列表")[0].path.clone();

    pair.b
        .resolve_conflict(&path, ConflictResolution::UseRemote)
        .expect("用远端解决");
    let status = pair.b.continue_rebase_and_push().expect("继续同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(porcelain(&pair.b), "");

    let item = item_of(&pair.b, &pair.item_id);
    assert!(item.body.contains("A 改的正文"), "{}", item.body);
    assert!(!item.body.contains("B 改的正文"), "{}", item.body);

    let status = pair.fixture.vault.sync().expect("A 同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(
        item_of(&pair.fixture.vault, &pair.item_id).body,
        item.body,
        "两端条目正文一致"
    );
    assert_eq!(
        pair.fixture.vault.list_items().expect("A 列出条目").len(),
        1
    );
}

/// 4b. 用本地：保留本地改动，继续 rebase + 推送后远端也变成本地版本。
#[test]
fn resolve_conflict_use_local() {
    let pair = conflicting_pair();
    let path = pair.b.sync_conflicts().expect("冲突列表")[0].path.clone();

    pair.b
        .resolve_conflict(&path, ConflictResolution::UseLocal)
        .expect("用本地解决");
    let status = pair.b.continue_rebase_and_push().expect("继续同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(porcelain(&pair.b), "");

    let item = item_of(&pair.b, &pair.item_id);
    assert!(item.body.contains("B 改的正文"), "{}", item.body);

    let status = pair.fixture.vault.sync().expect("A 同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(
        item_of(&pair.fixture.vault, &pair.item_id).body,
        item.body,
        "远端在 A 侧更新为本地版本"
    );
}

/// 4c. 保留双方：本地版留原文件，远端版另起新 id 的副本条目；两端都活下来、内容都不丢。
#[test]
fn resolve_conflict_keep_both() {
    let pair = conflicting_pair();
    let path = pair.b.sync_conflicts().expect("冲突列表")[0].path.clone();

    pair.b
        .resolve_conflict(&path, ConflictResolution::KeepBoth)
        .expect("保留双方");
    let status = pair.b.continue_rebase_and_push().expect("继续同步");
    assert_eq!(status.state, SyncState::Synced);
    assert_eq!(porcelain(&pair.b), "");

    let items = pair.b.list_items().expect("B 列出条目");
    assert_eq!(items.len(), 2, "冲突双方各成一条");
    let local = item_of(&pair.b, &pair.item_id);
    assert!(local.body.contains("B 改的正文"), "{}", local.body);
    let copy = items
        .iter()
        .find(|item| item.id != pair.item_id)
        .expect("副本条目");
    assert_ne!(copy.id, pair.item_id, "副本必须是新 id");
    assert!(copy.body.contains("A 改的正文"), "{}", copy.body);
    assert!(copy.title.contains("冲突副本"), "{}", copy.title);

    // A 同步后同样两条，id 与内容一一对应
    let status = pair.fixture.vault.sync().expect("A 同步");
    assert_eq!(status.state, SyncState::Synced);
    let a_items = pair.fixture.vault.list_items().expect("A 列出条目");
    assert_eq!(a_items.len(), 2, "{a_items:?}");
    assert_eq!(item_of(&pair.fixture.vault, &pair.item_id).body, local.body);
    assert_eq!(item_of(&pair.fixture.vault, &copy.id).body, copy.body);
}

/// 5. 未配置远端：sync 返回 NotConfigured，不报错、不炸。
#[test]
fn sync_without_remote_is_not_configured() {
    let (_dir, vault) = new_vault();

    let status = vault.sync().expect("未配置远端不应报错");
    assert_eq!(status.state, SyncState::NotConfigured);
    assert!(status.last_error.is_none());
    assert_eq!(
        vault.sync_status().expect("同步状态").state,
        SyncState::NotConfigured
    );
    assert!(vault.sync_conflicts().expect("冲突列表").is_empty());
}
