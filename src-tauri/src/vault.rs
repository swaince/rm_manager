use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};
use crate::model::{Item, ItemType, Space, Status};

/// vault 目录：条目文件放 `items/` 子目录，一条目一 Markdown 文件。
pub struct Vault {
    root: PathBuf,
}

/// 磁盘上的 frontmatter 形态；P0-01 只写最小字段集，其余字段后续票补。
#[derive(Serialize, Deserialize)]
struct Frontmatter {
    id: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    title: String,
    status: Status,
    space: Space,
    created: String,
}

impl Vault {
    /// 打开 vault（目录不存在则创建）。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("items"))?;
        Ok(Self { root })
    }

    pub fn items_dir(&self) -> PathBuf {
        self.root.join("items")
    }

    /// 新建条目：生成 id、写盘，返回内存模型（默认 status=inbox）。
    pub fn create_item(
        &self,
        title: &str,
        item_type: ItemType,
        space: Space,
        body: &str,
    ) -> Result<Item> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::InvalidTitle);
        }
        let now = OffsetDateTime::now_utc();
        let item = Item {
            id: self.next_id(item_type, now.year())?,
            item_type,
            title: title.to_string(),
            status: Status::Inbox,
            space,
            created: now.format(&Rfc3339)?,
            body: body.trim().to_string(),
        };
        let frontmatter = serde_norway::to_string(&Frontmatter {
            id: item.id.clone(),
            item_type: item.item_type,
            title: item.title.clone(),
            status: item.status,
            space: item.space,
            created: item.created.clone(),
        })?;
        let content = format!("---\n{frontmatter}---\n\n{}\n", item.body);
        fs::write(
            self.items_dir().join(file_name(&item.id, &item.title)),
            content,
        )?;
        Ok(item)
    }

    /// 列出全部条目，最新创建的在前。
    ///
    /// shortcut: items/ 里出现非条目 .md 会让整体报错；等有外部编辑器/导入场景再改为跳过并上报。
    pub fn list_items(&self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        for name in self.item_files()? {
            let raw = fs::read_to_string(self.items_dir().join(name))?;
            items.push(parse_item(&raw)?);
        }
        items.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| b.id.cmp(&a.id)));
        Ok(items)
    }

    /// id 规则 `TYPE-YYYY-NNNN`：取该类型现有文件的最大序号 +1（跨年份取全局最大，保证不撞号）。
    fn next_id(&self, item_type: ItemType, year: i32) -> Result<String> {
        let prefix = item_type.prefix();
        let mut max = 0u32;
        for name in self.item_files()? {
            let stem = name.split("--").next().unwrap_or_default();
            let mut parts = stem.split('-');
            if parts.next() != Some(prefix) {
                continue;
            }
            parts.next(); // 年份段不参与比较
            if let Some(seq) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
                max = max.max(seq);
            }
        }
        Ok(format!("{prefix}-{year:04}-{:04}", max + 1))
    }

    fn item_files(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(self.items_dir())? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") {
                names.push(name);
            }
        }
        Ok(names)
    }
}

/// 文件名 `ID--标题.md`；标题里的 Windows 非法字符与控制字符替换为 `-`。
fn file_name(id: &str, title: &str) -> String {
    let safe: String = title
        .chars()
        .map(|c| if is_illegal_in_file_name(c) { '-' } else { c })
        .collect();
    // Windows 不允许文件名以空格或点结尾
    let safe = safe.trim().trim_matches('.').trim();
    if safe.is_empty() {
        format!("{id}.md")
    } else {
        format!("{id}--{safe}.md")
    }
}

fn is_illegal_in_file_name(c: char) -> bool {
    c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

fn parse_item(raw: &str) -> Result<Item> {
    // 手工编辑的文件可能是 CRLF，统一成 LF 再解析
    let raw = raw.replace("\r\n", "\n");
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| Error::InvalidFrontmatter("缺少起始 ---".into()))?;
    let (yaml, body) = rest
        .split_once("\n---")
        .ok_or_else(|| Error::InvalidFrontmatter("缺少结束 ---".into()))?;
    let frontmatter: Frontmatter = serde_norway::from_str(yaml)?;
    Ok(Item {
        id: frontmatter.id,
        item_type: frontmatter.item_type,
        title: frontmatter.title,
        status: frontmatter.status,
        space: frontmatter.space,
        created: frontmatter.created,
        body: body.trim().to_string(),
    })
}
