use tauri::State;

use crate::error::Error;
use crate::model::{Item, ItemType, Space};
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
pub fn list_items(vault: State<'_, Vault>) -> Result<Vec<Item>, Error> {
    vault.list_items()
}
