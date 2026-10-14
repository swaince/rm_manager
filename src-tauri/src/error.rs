use std::fmt;

/// 核心层错误：vault 读写与 frontmatter 解析失败。
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Yaml(serde_norway::Error),
    Time(time::error::Format),
    InvalidFrontmatter(String),
    InvalidTitle,
    ItemNotFound(String),
    Db(rusqlite::Error),
    Json(serde_json::Error),
    ViewNotFound(String),
    InvalidViewName,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "vault 读写失败：{e}"),
            Error::Yaml(e) => write!(f, "frontmatter 解析失败：{e}"),
            Error::Time(e) => write!(f, "时间格式化失败：{e}"),
            Error::InvalidFrontmatter(msg) => write!(f, "frontmatter 格式错误：{msg}"),
            Error::InvalidTitle => write!(f, "条目标题不能为空"),
            Error::ItemNotFound(id) => write!(f, "条目不存在：{id}"),
            Error::Db(e) => write!(f, "索引数据库失败：{e}"),
            Error::Json(e) => write!(f, "条目序列化失败：{e}"),
            Error::ViewNotFound(name) => write!(f, "视图不存在：{name}"),
            Error::InvalidViewName => write!(f, "视图名不能为空"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_norway::Error> for Error {
    fn from(e: serde_norway::Error) -> Self {
        Error::Yaml(e)
    }
}

impl From<time::error::Format> for Error {
    fn from(e: time::error::Format) -> Self {
        Error::Time(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

/// Tauri 命令需要错误可序列化，前端只拿到人话消息。
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// 核心层统一返回类型。
pub type Result<T> = std::result::Result<T, Error>;
