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

    /// 新建时的默认状态：会话进行中，其余从收件箱开始。
    pub fn default_status(self) -> Status {
        match self {
            ItemType::Session => Status::Active,
            _ => Status::Inbox,
        }
    }
}

/// 条目状态；各类型各自的允许集合见 `allowed_statuses`，blocked / dropped 是全类型可用的旁路。
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
    // 想法保留；会话进行中 / 已收敛
    Kept,
    Active,
    Concluded,
}

/// 各类型允许的状态集合（FR-0.2）；blocked / dropped 旁路对全部类型可用。
pub fn allowed_statuses(item_type: ItemType) -> &'static [Status] {
    match item_type {
        ItemType::Idea => &[
            Status::Inbox,
            Status::Kept,
            Status::Blocked,
            Status::Dropped,
        ],
        ItemType::Requirement => &[
            Status::Inbox,
            Status::Clarifying,
            Status::Refining,
            Status::Ready,
            Status::Scheduled,
            Status::Doing,
            Status::Review,
            Status::Done,
            Status::Blocked,
            Status::Dropped,
        ],
        ItemType::Bug => &[
            Status::Inbox,
            Status::Ready,
            Status::Scheduled,
            Status::Doing,
            Status::Review,
            Status::Done,
            Status::Blocked,
            Status::Dropped,
        ],
        ItemType::Chore => &[
            Status::Inbox,
            Status::Ready,
            Status::Scheduled,
            Status::Doing,
            Status::Done,
            Status::Blocked,
            Status::Dropped,
        ],
        ItemType::Spike => &[
            Status::Inbox,
            Status::Doing,
            Status::Done,
            Status::Blocked,
            Status::Dropped,
        ],
        ItemType::Session => &[
            Status::Active,
            Status::Concluded,
            Status::Blocked,
            Status::Dropped,
        ],
    }
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
    /// 旁路字段：仅在 blocked / dropped 状态下存在，离开对应状态即清空
    pub blocked_reason: Option<String>,
    pub blocked_waiting_on: Option<String>,
    pub blocked_from: Option<Status>,
    pub dropped_reason: Option<String>,
    pub space: Space,
    /// 可选属性：未设置即 None / 空集合，frontmatter 不写对应的键
    pub project: Option<String>,
    pub priority: Option<Priority>,
    pub tags: Vec<String>,
    pub due: Option<String>,
    /// 拆解：父条目 id；顶层条目为 None
    pub parent: Option<String>,
    pub created: String,
    pub body: String,
}

impl Item {
    /// 离开 blocked / dropped 时清掉对应旁路字段。
    pub fn clear_status_extras(&mut self) {
        self.blocked_reason = None;
        self.blocked_waiting_on = None;
        self.blocked_from = None;
        self.dropped_reason = None;
    }
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

/// 父项聚合进度：total = 未 dropped 的直接子项数，done = 其中 status=done 的数量。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}
