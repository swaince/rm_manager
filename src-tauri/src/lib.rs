pub mod commands;
pub mod error;
pub mod model;
pub mod vault;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::vault::Vault;

/// 本地配置（app config dir 下的 config.json）；P0-01 只有 vault 路径。
#[derive(Serialize, Deserialize)]
struct Config {
    vault_path: PathBuf,
}

/// vault 路径：读本地配置；无配置则用「用户文档目录/rm-vault」并把配置写回。
fn resolve_vault_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = app.path().app_config_dir()?;
    let config_file = config_dir.join("config.json");
    if config_file.exists() {
        let config: Config = serde_json::from_str(&std::fs::read_to_string(&config_file)?)?;
        return Ok(config.vault_path);
    }
    let vault_path = app.path().document_dir()?.join("rm-vault");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(
        &config_file,
        serde_json::to_string_pretty(&Config {
            vault_path: vault_path.clone(),
        })?,
    )?;
    Ok(vault_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let vault = Vault::open(resolve_vault_path(app.handle())?)?;
            app.manage(vault);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_item,
            commands::list_items,
            commands::update_item,
            commands::set_parent,
            commands::children_of,
            commands::parent_progress
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
