use tauri::State;

use crate::error::Error;
use crate::index::{SavedView, SearchQuery};
use crate::model::{Item, ItemEdit, ItemType, Space};
use crate::vault::Vault;

// 命令层是薄胶水：只透传参数，业务在 vault / model 里。

#[tauri::command]
pub fn create_item(
    vault: State<'_, Vault>,
    title: String,
    item_type: ItemType,
    space: Space,
    body: String,
) -> Result<Item, Error> {
    vault.create_item(&title, item_type, space, &body)
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
