use rm_manager_lib::vault::Vault;
use tempfile::TempDir;

/// 测试夹具：临时 vault 目录。TempDir 必须存活到测试结束，drop 时目录自动清理。
pub fn new_vault() -> (TempDir, Vault) {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let vault = Vault::open(dir.path()).expect("打开 vault");
    (dir, vault)
}
