use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};
use crate::model::{Item, ItemEdit, ItemType, Priority, Progress, Space, Status};

/// vault 目录：条目文件放 `items/` 子目录，一条目一 Markdown 文件。
pub struct Vault {
    root: PathBuf,
}

/// 磁盘上的 frontmatter 形态：可选字段未设置时不写对应的键。
#[derive(Serialize, Deserialize)]
struct Frontmatter {
    id: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    title: String,
    status: Status,
    space: Space,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    priority: Option<Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    due: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
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
            project: None,
            priority: None,
            tags: Vec::new(),
            due: None,
            parent: None,
            created: now.format(&Rfc3339)?,
            body: body.trim().to_string(),
        };
        fs::write(
            self.items_dir().join(file_name(&item.id, &item.title)),
            render(&item)?,
        )?;
        Ok(item)
    }

    /// 更新条目：接收完整可编辑状态（清空 = None / 空集合），id / status / created 保持原值；
    /// 标题变化时同步重命名文件（id 不变，不会与其他条目撞名）。
    pub fn update_item(&self, id: &str, edit: &ItemEdit) -> Result<Item> {
        let title = edit.title.trim();
        if title.is_empty() {
            return Err(Error::InvalidTitle);
        }
        let old_path = self.item_path(id)?;
        let old = parse_item(&fs::read_to_string(&old_path)?)?;

        let item = Item {
            id: old.id,
            item_type: edit.item_type,
            title: title.to_string(),
            status: old.status,
            space: edit.space,
            project: optional_text(edit.project.as_deref()),
            priority: edit.priority,
            tags: cleaned_tags(&edit.tags),
            due: optional_text(edit.due.as_deref()),
            parent: old.parent,
            created: old.created,
            body: edit.body.trim().to_string(),
        };

        let new_path = self.items_dir().join(file_name(&item.id, &item.title));
        if new_path != old_path {
            fs::rename(&old_path, &new_path)?;
        }
        fs::write(&new_path, render(&item)?)?;
        Ok(item)
    }

    /// 拆解：给子条目设置 / 清空父条目（None 清空），返回更新后的子条目。
    ///
    /// 约束：父必须已存在、不能是自己、不能成环（沿提议父的祖先链上溯，遇到本条目即拒）；
    /// 层级深度只建议不强制（FR-4.2 建议 ≤2 层，超过不报错）。
    pub fn set_parent(&self, child_id: &str, parent: Option<&str>) -> Result<Item> {
        let path = self.item_path(child_id)?;
        let mut child = parse_item(&fs::read_to_string(&path)?)?;
        let parent = optional_text(parent);

        if let Some(parent_id) = parent.as_deref() {
            if parent_id == child.id {
                return Err(Error::InvalidParent("条目不能以自己为父".into()));
            }
            self.item_path(parent_id)?; // 父条目必须已存在
            // 沿父的祖先链上溯，遇到本条目即成环；祖先缺失（孤儿子项）或链上重复（手工编辑留下的旧环）即止。
            let mut current = Some(parent_id.to_string());
            let mut seen = std::collections::HashSet::new();
            while let Some(id) = current {
                if id == child.id {
                    return Err(Error::InvalidParent(format!(
                        "{} 已在 {parent_id} 的祖先链上，会形成循环",
                        child.id
                    )));
                }
                if !seen.insert(id.clone()) {
                    break;
                }
                current = match self.item_path(&id) {
                    Ok(ancestor_path) => parse_item(&fs::read_to_string(ancestor_path)?)?.parent,
                    Err(_) => None,
                };
            }
        }

        child.parent = parent;
        fs::write(&path, render(&child)?)?;
        Ok(child)
    }

    /// 直接子条目（不递归孙辈），按 created 升序；同一秒创建时按 id 升序保证顺序稳定。
    pub fn children_of(&self, id: &str) -> Result<Vec<Item>> {
        let mut children: Vec<Item> = self
            .list_items()?
            .into_iter()
            .filter(|item| item.parent.as_deref() == Some(id))
            .collect();
        children.sort_by(|a, b| a.created.cmp(&b.created).then_with(|| a.id.cmp(&b.id)));
        Ok(children)
    }

    /// 父项进度：只算直接子项，不递归孙辈；全部子项都 dropped 时是 0/0。
    pub fn parent_progress(&self, id: &str) -> Result<Progress> {
        let mut progress = Progress { done: 0, total: 0 };
        for child in self.children_of(id)? {
            if child.status == Status::Dropped {
                continue;
            }
            progress.total += 1;
            if child.status == Status::Done {
                progress.done += 1;
            }
        }
        Ok(progress)
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

    /// 按 id 找现有条目文件：文件名形如 `ID.md` 或 `ID--标题.md`。
    fn item_path(&self, id: &str) -> Result<PathBuf> {
        for name in self.item_files()? {
            let Some(rest) = name.strip_prefix(id) else {
                continue;
            };
            if rest == ".md" || (rest.starts_with("--") && rest.ends_with(".md")) {
                return Ok(self.items_dir().join(name));
            }
        }
        Err(Error::ItemNotFound(id.to_string()))
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

/// 渲染条目文件内容：frontmatter + 正文。
fn render(item: &Item) -> Result<String> {
    let frontmatter = serde_norway::to_string(&Frontmatter {
        id: item.id.clone(),
        item_type: item.item_type,
        title: item.title.clone(),
        status: item.status,
        space: item.space,
        project: item.project.clone(),
        priority: item.priority,
        due: item.due.clone(),
        tags: item.tags.clone(),
        parent: item.parent.clone(),
        created: item.created.clone(),
    })?;
    Ok(format!("---\n{frontmatter}---\n\n{}\n", item.body))
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
        project: frontmatter.project,
        priority: frontmatter.priority,
        tags: frontmatter.tags,
        due: frontmatter.due,
        parent: frontmatter.parent,
        created: frontmatter.created,
        body: body.trim().to_string(),
    })
}

/// 可选文本字段：trim 后为空视为「清空」，写盘时省略该键。
fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// 标签去掉空白项并保持原顺序；全空 = 空集合，写盘时省略该键。
fn cleaned_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}
