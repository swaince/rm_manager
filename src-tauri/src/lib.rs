pub mod capture;
pub mod commands;
pub mod error;
pub mod git;
pub mod index;
pub mod model;
pub mod sync;
pub mod vault;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent};

use crate::sync::{SYNC_STATUS_EVENT, SyncStatus};
use crate::vault::Vault;

/// 迷你速记窗的 window label（tauri.conf.json 预置、启动时隐藏、同一 index.html）。
const CAPTURE_LABEL: &str = "capture";
/// 主窗 label（tauri.conf.json 首个窗口的默认值）。
const MAIN_LABEL: &str = "main";

/// 托盘菜单：常驻的「新建速记」入口，覆盖不想用快捷键的场景。
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let quick_capture = MenuItem::with_id(app, "quick_capture", "新建速记", true, None::<&str>)?;
    let show_main = MenuItem::with_id(app, "show_main", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quick_capture, &show_main, &quit])?;

    let mut tray = TrayIconBuilder::with_id("tray")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quick_capture" => commands::reveal_window(app, CAPTURE_LABEL),
            "show_main" => commands::reveal_window(app, MAIN_LABEL),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

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

/// 定时同步间隔（FR-9.1）：启动拉一次、每 15 分钟同步一次、退出前推一次。
const SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// 退出推送的最长等待：超时直接放行退出（推送是原子的，杀掉不留半个提交）。
const EXIT_PUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// 退出推送只启动一次：ExitRequested 在真正退出前还会来一次。
static EXIT_PUSH_STARTED: AtomicBool = AtomicBool::new(false);

/// 同步一次并把状态广播给前端（角标与冲突对话框据此刷新）；`on_exit` 走带超时的退出推送。
fn sync_now(app: &AppHandle, on_exit: bool) {
    let vault = app.state::<Vault>();
    let status = if on_exit {
        vault.push_on_exit(EXIT_PUSH_TIMEOUT)
    } else {
        vault.sync()
    };
    let payload = status.unwrap_or_else(|error| SyncStatus::failed(error.to_string()));
    let _ = app.emit(SYNC_STATUS_EVENT, payload);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // 全局快捷键在 Rust 侧注册 / 触发，前端只负责绑定行为
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let vault = Vault::open(resolve_vault_path(app.handle())?)?;
            app.manage(vault);
            // 托盘失败不挡启动：主窗入口与快捷键都能覆盖
            if let Err(error) = setup_tray(app.handle()) {
                eprintln!("托盘初始化失败：{error}");
            }
            // 启动拉取一次，之后每 15 分钟同步一次：都在后台跑，不挡启动
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || sync_now(&handle, false));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(SYNC_INTERVAL).await;
                    let handle = handle.clone();
                    let _ = tauri::async_runtime::spawn_blocking(move || sync_now(&handle, false))
                        .await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_item,
            commands::harvest_clipboard,
            commands::open_capture_window,
            commands::hide_capture_window,
            commands::focus_main_window,
            commands::list_items,
            commands::update_item,
            commands::search_items,
            commands::rebuild_index,
            commands::list_views,
            commands::save_view,
            commands::delete_view,
            commands::set_status,
            commands::block_item,
            commands::unblock_item,
            commands::drop_item,
            commands::waive_dor,
            commands::set_parent,
            commands::children_of,
            commands::item_timeline,
            commands::parent_progress,
            commands::schedule_item,
            commands::set_focus,
            commands::sync,
            commands::sync_status,
            commands::set_remote,
            commands::test_remote,
            commands::sync_conflicts,
            commands::resolve_conflict,
            commands::continue_sync,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                // 退出前推一次：只拦截一次，推完（或 10 秒超时）再真正退出
                if EXIT_PUSH_STARTED.swap(true, Ordering::SeqCst) {
                    return;
                }
                api.prevent_exit();
                let handle = app.clone();
                std::thread::spawn(move || {
                    sync_now(&handle, true);
                    handle.exit(0);
                });
            }
        });
}
