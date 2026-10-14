//! 捕获：迷你速记窗与剪贴板收割共用的落库形态。
//! 核心不允许空标题：标题为空时取正文首个非空行（trim 后截 80 字符），正文全空用兜底标题。

use crate::error::Result;
use crate::model::{Item, ItemType, Space};
use crate::vault::Vault;

/// 速记窗标题与正文都空时的兜底标题
pub const QUICK_NOTE_FALLBACK: &str = "速记";
/// 剪贴板收割正文全空时的兜底标题
pub const HARVEST_FALLBACK: &str = "剪贴板收割";
/// 派生标题的最大字符数（按字符，不按字节）
const TITLE_MAX_CHARS: usize = 80;

impl Vault {
    /// 迷你速记窗落库：标题 / 正文均可空，类型与空间由表单选（默认 idea / personal）；
    /// 状态取类型默认（idea → inbox）。
    pub fn capture_item(
        &self,
        title: &str,
        item_type: ItemType,
        space: Space,
        body: &str,
    ) -> Result<Item> {
        self.create_item(
            &derive_title(title, body, QUICK_NOTE_FALLBACK),
            item_type,
            space,
            body,
        )
    }

    /// 剪贴板收割：整段文本落为 idea / personal / inbox；正文存全文（含换行），标题取首行派生。
    pub fn harvest_clipboard(&self, text: &str) -> Result<Item> {
        self.create_item(
            &derive_title("", text, HARVEST_FALLBACK),
            ItemType::Idea,
            Space::Personal,
            text,
        )
    }
}

/// 派生标题：标题 trim 后非空即用它；否则取正文首个非空行 trim 后截 80 字符；都没有用 fallback。
fn derive_title(title: &str, body: &str, fallback: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.to_string();
    }
    match body.lines().map(str::trim).find(|line| !line.is_empty()) {
        Some(line) => line.chars().take(TITLE_MAX_CHARS).collect(),
        None => fallback.to_string(),
    }
}
