use tauri::{Manager, State};

use crate::error::Error;
use crate::git::TimelineEntry;
use crate::index::{SavedView, SearchQuery};
use crate::model::{Item, ItemEdit, ItemType, Progress, Space, Status};
use crate::vault::Vault;

// 命令层是薄胶水：只透传参数，业务在 vault / model 里。

/// 新建条目；标题可为空 —— 迷你速记窗走同一命令，空标题由核心从正文派生（全空用「速记」）。
#[tauri::command]
pub fn create_item(
    vault: State<'_, Vault>,
    title: String,
    item_type: ItemType,
    space: Space,
    body: String,
) -> Result<Item, Error> {
    vault.capture_item(&title, item_type, space, &body)
}

/// 剪贴板收割：正文整段落为 inbox 条目（读剪贴板在胶水层，落库形态在核心）。
#[tauri::command]
pub fn harvest_clipboard(vault: State<'_, Vault>, text: String) -> Result<Item, Error> {
    vault.harvest_clipboard(&text)
}

/// 显示并聚焦窗口（托盘菜单与速记入口共用）。
pub(crate) fn reveal_window(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 打开迷你速记窗（全局快捷键 / 主窗常驻入口）。
#[tauri::command]
pub fn open_capture_window(app: tauri::AppHandle) {
    reveal_window(&app, "capture");
}

/// 收起迷你速记窗（Esc、取消与提交后都走它）。
#[tauri::command]
pub fn hide_capture_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("capture") {
        let _ = window.hide();
    }
}

/// 把焦点交还主窗（速记提交后）。
#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) {
    reveal_window(&app, "main");
}

#[tauri::command]
pub fn update_item(vault: State<'_, Vault>, id: String, edit: ItemEdit) -> Result<Item, Error> {
    vault.update_item(&id, &edit)
}

#[tauri::command]
pub fn list_items(vault: State<'_, Vault>) -> Result<Vec<Item>, Error> {
    vault.list_items()
}

#[tauri::command]
pub fn search_items(vault: State<'_, Vault>, query: SearchQuery) -> Result<Vec<Item>, Error> {
    vault.search(&query)
}

#[tauri::command]
pub fn rebuild_index(vault: State<'_, Vault>) -> Result<(), Error> {
    vault.rebuild_index()
}

#[tauri::command]
pub fn list_views(vault: State<'_, Vault>) -> Result<Vec<SavedView>, Error> {
    vault.list_views()
}

#[tauri::command]
pub fn save_view(
    vault: State<'_, Vault>,
    name: String,
    query: SearchQuery,
) -> Result<SavedView, Error> {
    vault.save_view(&name, &query)
}

#[tauri::command]
pub fn delete_view(vault: State<'_, Vault>, name: String) -> Result<(), Error> {
    vault.delete_view(&name)
}

#[tauri::command]
pub fn set_status(vault: State<'_, Vault>, id: String, status: Status) -> Result<Item, Error> {
    vault.set_status(&id, status)
}

#[tauri::command]
pub fn block_item(
    vault: State<'_, Vault>,
    id: String,
    reason: String,
    waiting_on: String,
) -> Result<Item, Error> {
    vault.block_item(&id, &reason, &waiting_on)
}

#[tauri::command]
pub fn unblock_item(vault: State<'_, Vault>, id: String) -> Result<Item, Error> {
    vault.unblock_item(&id)
}

#[tauri::command]
pub fn drop_item(vault: State<'_, Vault>, id: String, reason: String) -> Result<Item, Error> {
    vault.drop_item(&id, &reason)
}

/// 豁免就绪检查：理由必填并留痕。
#[tauri::command]
pub fn waive_dor(vault: State<'_, Vault>, id: String, reason: String) -> Result<Item, Error> {
    vault.waive_dor(&id, &reason)
}

#[tauri::command]
pub fn set_parent(
    vault: State<'_, Vault>,
    id: String,
    parent: Option<String>,
) -> Result<Item, Error> {
    vault.set_parent(&id, parent.as_deref())
}

#[tauri::command]
pub fn children_of(vault: State<'_, Vault>, id: String) -> Result<Vec<Item>, Error> {
    vault.children_of(&id)
}

#[tauri::command]
pub fn parent_progress(vault: State<'_, Vault>, id: String) -> Result<Progress, Error> {
    vault.parent_progress(&id)
}
#[tauri::command]
pub fn item_timeline(vault: State<'_, Vault>, id: String) -> Result<Vec<TimelineEntry>, Error> {
    vault.item_timeline(&id)
}

#[tauri::command]
pub fn schedule_item(
    vault: State<'_, Vault>,
    id: String,
    week: Option<String>,
) -> Result<Item, Error> {
    vault.schedule_item(&id, week.as_deref())
}

#[tauri::command]
pub fn set_focus(vault: State<'_, Vault>, id: String, date: Option<String>) -> Result<Item, Error> {
    vault.set_focus(&id, date.as_deref())
}
