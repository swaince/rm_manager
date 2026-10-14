use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};
use crate::git;
use crate::model::{
    AcceptanceCriterion, Clarification, Estimate, Item, ItemEdit, ItemType, Priority, Progress,
    Space, Status, allowed_statuses,
};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blocked_waiting_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blocked_from: Option<Status>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dropped_reason: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scheduled_week: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    focus_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    estimate: Option<Estimate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    acceptance: Vec<AcceptanceCriterion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    clarifications: Vec<Clarification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dor_waived: Option<String>,
    created: String,
}

impl Vault {
    /// 打开 vault（目录不存在则创建）；没有 .git 就 git init（并补上缺失的提交身份），索引是本地派生物，根 .gitignore 一并补上忽略项。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("items"))?;
        ensure_gitignores_index(&root)?;
        let _ = git::init_if_needed(&root); // git 缺失只影响留痕，不影响使用
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn items_dir(&self) -> PathBuf {
        self.root.join("items")
    }

    /// 新建条目：生成 id、写盘，返回内存模型（状态按类型取默认值）。
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
            status: item_type.default_status(),
            blocked_reason: None,
            blocked_waiting_on: None,
            blocked_from: None,
            dropped_reason: None,
            space,
            project: None,
            priority: None,
            tags: Vec::new(),
            due: None,
            parent: None,
            scheduled_week: None,
            focus_on: None,
            estimate: None,
            acceptance: Vec::new(),
            clarifications: Vec::new(),
            dor_waived: None,
            created: now.format(&Rfc3339)?,
            body: body.trim().to_string(),
        };
        fs::write(
            self.items_dir().join(file_name(&item.id, &item.title)),
            render(&item)?,
        )?;
        self.commit(&git::create_message(&item.id));
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

        let mut item = Item {
            id: old.id,
            item_type: edit.item_type,
            title: title.to_string(),
            status: old.status,
            blocked_reason: old.blocked_reason,
            blocked_waiting_on: old.blocked_waiting_on,
            blocked_from: old.blocked_from,
            dropped_reason: old.dropped_reason,
            space: edit.space,
            project: optional_text(edit.project.as_deref()),
            priority: edit.priority,
            tags: cleaned_tags(&edit.tags),
            due: optional_text(edit.due.as_deref()),
            parent: old.parent,
            scheduled_week: old.scheduled_week,
            focus_on: old.focus_on,
            estimate: edit.estimate,
            acceptance: cleaned_acceptance(&edit.acceptance),
            clarifications: cleaned_clarifications(&edit.clarifications),
            // 豁免理由只能由 waive_dor 写入与留痕，编辑正文不重置
            dor_waived: old.dor_waived,
            created: old.created,
            body: edit.body.trim().to_string(),
        };

        // 改类型后旧状态可能不在新类型的集合里：回到新类型默认状态并清掉旁路字段
        if !allowed_statuses(item.item_type).contains(&item.status) {
            item.status = item.item_type.default_status();
            item.clear_status_extras();
        }

        let new_path = self.items_dir().join(file_name(&item.id, &item.title));
        if new_path != old_path {
            fs::rename(&old_path, &new_path)?;
        }
        fs::write(&new_path, render(&item)?)?;
        self.commit(&git::update_message(&item.id));
        Ok(item)
    }

    /// 改状态：目标必须是该类型允许集合内且非旁路状态（blocked / dropped 走专门动作）；
    /// 需求进 ready 还要过就绪检查（FR-4.3），豁免过的放行。
    pub fn set_status(&self, id: &str, status: Status) -> Result<Item> {
        let mut item = self.read_item(id)?;
        if matches!(status, Status::Blocked | Status::Dropped) {
            return Err(Error::InvalidStatus(
                "阻塞与放弃必须填写原因，请用对应的动作".into(),
            ));
        }
        if !allowed_statuses(item.item_type).contains(&status) {
            return Err(Error::InvalidStatus(format!(
                "{:?} 类型没有 {status:?} 状态",
                item.item_type
            )));
        }
        let from = item.status;
        if item.item_type == ItemType::Requirement
            && status == Status::Ready
            && item.dor_waived.is_none()
        {
            let gaps = dor_gaps_of(&item);
            if !gaps.is_empty() {
                return Err(Error::InvalidStatus(format!(
                    "就绪检查未通过：{}",
                    gaps.join("；")
                )));
            }
        }
        item.status = status;
        item.clear_status_extras();
        self.write_item(&item)?;
        self.commit(&git::status_message(&item.id, from, item.status));
        Ok(item)
    }

    /// 就绪检查（FR-4.3）未满足项，按 验收标准 → 估算 → 澄清 固定顺序；空 = 可进 ready。
    pub fn dor_gaps(&self, id: &str) -> Result<Vec<String>> {
        Ok(dor_gaps_of(&self.read_item(id)?))
    }

    /// 豁免就绪检查：理由必填，写入 frontmatter 留痕；豁免后该需求进 ready 免检。
    pub fn waive_dor(&self, id: &str, reason: &str) -> Result<Item> {
        let mut item = self.read_item(id)?;
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Error::InvalidStatus("豁免就绪检查必须填写理由".into()));
        }
        item.dor_waived = Some(reason.to_string());
        self.write_item(&item)?;
        self.commit(&git::update_message(&item.id));
        Ok(item)
    }

    /// 阻塞：必填原因与等待对象，并记下当前状态；已阻塞或已放弃的条目拒绝。
    pub fn block_item(&self, id: &str, reason: &str, waiting_on: &str) -> Result<Item> {
        let mut item = self.read_item(id)?;
        match item.status {
            Status::Blocked => return Err(Error::InvalidStatus("条目已处于阻塞状态".into())),
            Status::Dropped => return Err(Error::InvalidStatus("已放弃的条目不再阻塞".into())),
            _ => {}
        }
        let (reason, waiting_on) = (reason.trim(), waiting_on.trim());
        if reason.is_empty() || waiting_on.is_empty() {
            return Err(Error::InvalidStatus("阻塞必须填写原因与等待对象".into()));
        }
        let from = item.status;
        item.blocked_from = Some(from);
        item.blocked_reason = Some(reason.to_string());
        item.blocked_waiting_on = Some(waiting_on.to_string());
        item.status = Status::Blocked;
        self.write_item(&item)?;
        self.commit(&git::status_message(&item.id, from, item.status));
        Ok(item)
    }

    /// 解除阻塞：回到阻塞前的状态（记录缺失或对新类型不合法时，回到类型默认状态）。
    pub fn unblock_item(&self, id: &str) -> Result<Item> {
        let mut item = self.read_item(id)?;
        if item.status != Status::Blocked {
            return Err(Error::InvalidStatus("条目当前未被阻塞".into()));
        }
        let from = item.status;
        let restored = item.blocked_from.unwrap_or(item.item_type.default_status());
        item.status = if allowed_statuses(item.item_type).contains(&restored) {
            restored
        } else {
            item.item_type.default_status()
        };
        item.blocked_reason = None;
        item.blocked_waiting_on = None;
        item.blocked_from = None;
        self.write_item(&item)?;
        self.commit(&git::status_message(&item.id, from, item.status));
        Ok(item)
    }

    /// 放弃：必填原因；离开 blocked 时清掉阻塞字段。
    pub fn drop_item(&self, id: &str, reason: &str) -> Result<Item> {
        let mut item = self.read_item(id)?;
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Error::InvalidStatus("放弃条目必须填写原因".into()));
        }
        let from = item.status;
        item.status = Status::Dropped;
        item.dropped_reason = Some(reason.to_string());
        item.blocked_reason = None;
        item.blocked_waiting_on = None;
        item.blocked_from = None;
        self.write_item(&item)?;
        self.commit(&git::status_message(&item.id, from, item.status));
        Ok(item)
    }

    /// 写盘后自动本地提交（消息格式见 `docs/sync-architecture.md` §4）；
    /// git 不可用时不提交也不报错：留痕是附加能力，不能挡住保存。
    fn commit(&self, message: &str) {
        let _ = git::commit_all(&self.root, message);
    }

    fn read_item(&self, id: &str) -> Result<Item> {
        parse_item(&fs::read_to_string(self.item_path(id)?)?)
    }

    /// 写回条目文件；路径由 id + 标题算出，状态类操作不改标题，路径不变。
    fn write_item(&self, item: &Item) -> Result<()> {
        fs::write(
            self.items_dir().join(file_name(&item.id, &item.title)),
            render(item)?,
        )?;
        Ok(())
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
        self.commit(&git::parent_message(&child.id, child.parent.as_deref()));
        Ok(child)
    }

    /// 排期：排入某一周（week = 该周周一的日期，由调用方算好）或清空（None / 空白串）。
    /// 日期串原样存储、不做格式校验（与 due 一致），格式由调用方保证。
    pub fn schedule_item(&self, id: &str, week: Option<&str>) -> Result<Item> {
        let mut item = self.read_item(id)?;
        item.scheduled_week = optional_text(week);
        self.write_item(&item)?;
        self.commit(&git::update_message(&item.id));
        Ok(item)
    }

    /// 今日焦点：把条目选进某天的焦点（date = `YYYY-MM-DD`）或清空（None / 空白串、空串）。
    pub fn set_focus(&self, id: &str, date: Option<&str>) -> Result<Item> {
        let mut item = self.read_item(id)?;
        item.focus_on = optional_text(date);
        self.write_item(&item)?;
        self.commit(&git::update_message(&item.id));
        Ok(item)
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
    pub(crate) fn item_path(&self, id: &str) -> Result<PathBuf> {
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
/// vault 根 .gitignore 必须忽略索引目录，索引绝不进同步；已有内容原样保留。
fn ensure_gitignores_index(root: &Path) -> Result<()> {
    let path = root.join(".gitignore");
    let existing = if path.is_file() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    if existing
        .lines()
        .any(|line| matches!(line.trim(), ".rm-index/" | ".rm-index"))
    {
        return Ok(());
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(".rm-index/\n");
    fs::write(&path, text)?;
    Ok(())
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

/// 文件名非法字符（视图文件名同样用它）。
pub(crate) fn is_illegal_in_file_name(c: char) -> bool {
    c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

/// 渲染条目文件内容：frontmatter + 正文。
fn render(item: &Item) -> Result<String> {
    let frontmatter = serde_norway::to_string(&Frontmatter {
        id: item.id.clone(),
        item_type: item.item_type,
        title: item.title.clone(),
        status: item.status,
        blocked_reason: item.blocked_reason.clone(),
        blocked_waiting_on: item.blocked_waiting_on.clone(),
        blocked_from: item.blocked_from,
        dropped_reason: item.dropped_reason.clone(),
        space: item.space,
        project: item.project.clone(),
        priority: item.priority,
        due: item.due.clone(),
        tags: item.tags.clone(),
        parent: item.parent.clone(),
        scheduled_week: item.scheduled_week.clone(),
        focus_on: item.focus_on.clone(),
        estimate: item.estimate,
        acceptance: item.acceptance.clone(),
        clarifications: item.clarifications.clone(),
        dor_waived: item.dor_waived.clone(),
        created: item.created.clone(),
    })?;
    Ok(format!("---\n{frontmatter}---\n\n{}\n", item.body))
}

/// 解析条目文件（索引增量更新也用它）。
pub(crate) fn parse_item(raw: &str) -> Result<Item> {
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
        blocked_reason: frontmatter.blocked_reason,
        blocked_waiting_on: frontmatter.blocked_waiting_on,
        blocked_from: frontmatter.blocked_from,
        dropped_reason: frontmatter.dropped_reason,
        space: frontmatter.space,
        project: frontmatter.project,
        priority: frontmatter.priority,
        tags: frontmatter.tags,
        due: frontmatter.due,
        parent: frontmatter.parent,
        scheduled_week: frontmatter.scheduled_week,
        focus_on: frontmatter.focus_on,
        estimate: frontmatter.estimate,
        acceptance: frontmatter.acceptance,
        clarifications: frontmatter.clarifications,
        dor_waived: frontmatter.dor_waived,
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

/// 就绪检查（FR-4.3）未满足项，按 验收标准 → 估算 → 澄清 固定顺序。
fn dor_gaps_of(item: &Item) -> Vec<String> {
    let mut gaps = Vec::new();
    if item.acceptance.is_empty() {
        gaps.push("验收标准未填写".to_string());
    } else {
        let undone = item
            .acceptance
            .iter()
            .filter(|criterion| !criterion.done)
            .count();
        if undone > 0 {
            gaps.push(format!("验收标准还有 {undone} 条未勾选"));
        }
    }
    if item.estimate.is_none() {
        gaps.push("未估算".to_string());
    }
    let unanswered = item
        .clarifications
        .iter()
        .filter(|clarification| clarification.answer.is_none())
        .count();
    if unanswered > 0 {
        gaps.push(format!("澄清问题还有 {unanswered} 条待答复"));
    }
    gaps
}

/// 验收标准：空白文本项丢掉，其余 trim；全空 = 空集合，写盘时省略该键。
fn cleaned_acceptance(criteria: &[AcceptanceCriterion]) -> Vec<AcceptanceCriterion> {
    criteria
        .iter()
        .filter_map(|criterion| {
            let text = criterion.text.trim();
            if text.is_empty() {
                return None;
            }
            Some(AcceptanceCriterion {
                text: text.to_string(),
                done: criterion.done,
            })
        })
        .collect()
}

/// 澄清记录：空白问题项丢掉，答复与来源人 trim 后为空即「未填」。
fn cleaned_clarifications(clarifications: &[Clarification]) -> Vec<Clarification> {
    clarifications
        .iter()
        .filter_map(|clarification| {
            let question = clarification.question.trim();
            if question.is_empty() {
                return None;
            }
            Some(Clarification {
                question: question.to_string(),
                answer: optional_text(clarification.answer.as_deref()),
                source: optional_text(clarification.source.as_deref()),
            })
        })
        .collect()
}
