use tauri::State;

use crate::error::Error;
use crate::model::{Item, ItemEdit, ItemType, Space, Status};
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
