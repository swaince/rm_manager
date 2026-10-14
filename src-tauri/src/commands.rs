use tauri::State;

use crate::error::Error;
use crate::model::{Item, ItemEdit, ItemType, Progress, Space};
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
