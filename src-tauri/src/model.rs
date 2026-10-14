use serde::{Deserialize, Serialize};

/// 条目类型，固定六个值；序列化值与 frontmatter 的 type 字段一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Idea,
    Requirement,
    Bug,
    Spike,
    Chore,
    Session,
}

impl ItemType {
    /// id 前缀，与类型固定映射（见 docs/sync-architecture.md）
    pub fn prefix(self) -> &'static str {
        match self {
            ItemType::Idea => "IDEA",
            ItemType::Requirement => "REQ",
            ItemType::Bug => "BUG",
            ItemType::Spike => "SPIKE",
            ItemType::Chore => "CHORE",
            ItemType::Session => "SESS",
        }
    }
}

/// 条目状态；各类型有各自的状态集合，P0-01 只写入默认值 inbox。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Inbox,
    Clarifying,
    Refining,
    Ready,
    Scheduled,
    Doing,
    Blocked,
    Review,
    Done,
    Dropped,
}

/// 空间：条目的顶层归属，固定 work / personal。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Space {
    Work,
    Personal,
}

/// 条目：一条目一 Markdown 文件，属性在 frontmatter，正文是 Markdown。
/// `created` 是 RFC3339 字符串（UTC）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub status: Status,
    pub space: Space,
    pub created: String,
    pub body: String,
}
