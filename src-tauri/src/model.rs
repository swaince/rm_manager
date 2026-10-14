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
/// 条目：一条目一 Markdown 文件，属性在 frontmatter，正文是 Markdown。
/// `created` 是 RFC3339 字符串（UTC）；Deserialize 供索引把它整条存成 JSON。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    pub status: Status,
    pub space: Space,
    /// 可选属性：未设置即 None / 空集合，frontmatter 不写对应的键
    pub project: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub due: Option<String>,
    pub created: String,
    pub body: String,
}

/// 优先级：人工判定的 P0–P3 等级，frontmatter 序列化为 `P0`–`P3`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

/// 编辑载荷：条目的完整可编辑状态，清空 = None / 空集合；
/// id / status / created 不在可编辑范围内，由核心保持原值。
#[derive(Debug, Clone, Deserialize)]
pub struct ItemEdit {
    pub title: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub space: Space,
    pub body: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub due: Option<String>,
}
