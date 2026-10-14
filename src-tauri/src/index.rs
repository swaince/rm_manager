use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::types::Value;
use rusqlite::{Connection, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{Item, ItemType, Priority, Space, Status};
use crate::vault::{Vault, is_illegal_in_file_name, parse_item};

/// 索引结构版本：结构一变，旧的库会被判为过期并全量重建。
const SCHEMA_VERSION: i64 = 1;

/// 索引表：结构化字段供筛选，title / body 供 FTS5 全文匹配，data 存整条 Item 的 JSON。
///
/// shortcut: 筛选列是 FTS5 的 UNINDEXED 列，按它们过滤要全表扫描；个人量级（几千条）够用，
/// 真卡顿了再拆普通表 + 索引。
const SCHEMA: &str = "
CREATE VIRTUAL TABLE items_fts USING fts5(
  id UNINDEXED, file UNINDEXED, mtime UNINDEXED, size UNINDEXED, hash UNINDEXED,
  title, body,
  type UNINDEXED, status UNINDEXED, space UNINDEXED, project UNINDEXED,
  priority UNINDEXED, tags UNINDEXED, due UNINDEXED, created UNINDEXED,
  data UNINDEXED,
  tokenize = 'trigram'
);
PRAGMA user_version = 1;
";

/// 搜索条件：字段名与保存视图的 YAML 一致；空条件 = 不过滤。
/// 多值字段里 types / statuses / priority 取「任一命中」，tags 取「全部命中」。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 全文查询：命中标题或正文；3 字符以上走 FTS5 trigram，更短的走子串扫描
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<ItemType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<Status>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priority: Vec<Priority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<Space>,
    /// 目标日期区间，含端点；未排期（无 due）的条目不会命中
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_after: Option<String>,
}

/// 保存的视图：name 即文件名（`views/<name>.yaml`），query 是完整搜索条件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedView {
    pub name: String,
    pub query: SearchQuery,
}

impl Vault {
    /// 搜索：先校验 / 补齐索引，再按条件查；空条件 = 全部条目，结果按 created 降序。
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<Item>> {
        let conn = self.checked_index()?;
        let (sql, params) = build_sql(query);
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(params))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(serde_json::from_str(&row.get::<_, String>(0)?)?);
        }
        Ok(items)
    }

    /// 全量重建索引（删掉旧库，items/ 全部重解析）。
    pub fn rebuild_index(&self) -> Result<()> {
        self.rebuild().map(drop)
    }

    /// 打开 / 搜索前的校验：库缺失、损坏、条目文件增删改都在这里补齐。
    pub fn ensure_index(&self) -> Result<()> {
        self.checked_index().map(drop)
    }

    /// 可用的索引连接：增量校验失败或结构过期，就退到一次全量重建。
    fn checked_index(&self) -> Result<Connection> {
        match self.sync_index() {
            Ok(Some(conn)) => Ok(conn),
            _ => self.rebuild(),
        }
    }

    /// 增量补齐现有库；库不存在 / 损坏 / 结构过期 → Ok(None)，交给调用方全量重建。
    fn sync_index(&self) -> Result<Option<Connection>> {
        let Some(conn) = self.open_existing_index()? else {
            return Ok(None);
        };
        sync_items(&conn, &self.items_dir())?;
        Ok(Some(conn))
    }

    fn open_existing_index(&self) -> Result<Option<Connection>> {
        let path = index_db_path(self.root());
        if !path.is_file() {
            return Ok(None);
        }
        let conn = Connection::open(&path)?;
        // 结构版本读不出来 = 文件不是（或不再是）SQLite 库，按过期处理
        let version: rusqlite::Result<i64> =
            conn.query_row("PRAGMA user_version", [], |row| row.get(0));
        Ok(match version {
            Ok(SCHEMA_VERSION) => Some(conn),
            _ => None,
        })
    }

    /// 删库重开：结构按 SCHEMA 建，内容按 items/ 全部重解析。
    fn rebuild(&self) -> Result<Connection> {
        let path = index_db_path(self.root());
        fs::create_dir_all(path.parent().expect("索引路径有父目录"))?;
        let _ = fs::remove_file(&path); // 删失败也无妨：建表会先失败并把真实错误抛出来
        let conn = Connection::open(&path)?;
        conn.execute_batch(SCHEMA)?;
        sync_items(&conn, &self.items_dir())?;
        Ok(conn)
    }

    /// 保存视图到 `views/<名字>.yaml`；同名覆盖。
    pub fn save_view(&self, name: &str, query: &SearchQuery) -> Result<SavedView> {
        let slug = view_slug(name)?;
        let dir = self.views_dir();
        fs::create_dir_all(&dir)?;
        fs::write(
            dir.join(format!("{slug}.yaml")),
            serde_norway::to_string(query)?,
        )?;
        Ok(SavedView {
            name: slug,
            query: query.clone(),
        })
    }

    /// 全部保存视图，按名字排序（重启后从 views/ 目录读回来）。
    pub fn list_views(&self) -> Result<Vec<SavedView>> {
        let dir = self.views_dir();
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut views = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            views.push(SavedView {
                name: name.to_string(),
                query: serde_norway::from_str(&fs::read_to_string(&path)?)?,
            });
        }
        views.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(views)
    }

    /// 删除视图；视图不存在时报错。
    pub fn delete_view(&self, name: &str) -> Result<()> {
        let path = self.views_dir().join(format!("{}.yaml", view_slug(name)?));
        if !path.is_file() {
            return Err(Error::ViewNotFound(name.to_string()));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    fn views_dir(&self) -> PathBuf {
        self.root().join("views")
    }
}

/// 索引数据库文件：vault 根下的 `.rm-index/index.db`（Vault::open 保证它被 .gitignore 忽略）。
fn index_db_path(root: &Path) -> PathBuf {
    root.join(".rm-index").join("index.db")
}

/// 视图名 → 文件名（非法字符替换为 `-`），空名字拒绝。
fn view_slug(name: &str) -> Result<String> {
    let slug: String = name
        .trim()
        .chars()
        .map(|c| if is_illegal_in_file_name(c) { '-' } else { c })
        .collect();
    let slug = slug.trim().trim_matches('.').trim();
    if slug.is_empty() {
        return Err(Error::InvalidViewName);
    }
    Ok(slug.to_string())
}

/// 把索引补齐到与 items/ 一致：mtime + size 没变就跳过；变了再比哈希，内容真变了才重解析。
/// 文件没了（删除或改名）的索引行一并移除。
fn sync_items(conn: &Connection, items_dir: &Path) -> Result<()> {
    let mut indexed: HashMap<String, (i64, i64, i64)> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT file, mtime, size, hash FROM items_fts")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            indexed.insert(row.get(0)?, (row.get(1)?, row.get(2)?, row.get(3)?));
        }
    }

    let mut present = HashSet::new();
    for entry in fs::read_dir(items_dir)? {
        let path = entry?.path();
        let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file.ends_with(".md") {
            continue;
        }
        present.insert(file.to_string());

        let meta = fs::metadata(&path)?;
        let mtime = mtime_ns(&meta);
        let size = meta.len() as i64;
        if let Some(&(old_mtime, old_size, _)) = indexed.get(file)
            && old_mtime == mtime
            && old_size == size
        {
            continue;
        }

        let raw = fs::read_to_string(&path)?;
        let hash = fnv1a(raw.as_bytes());
        if let Some(&(_, _, old_hash)) = indexed.get(file)
            && old_hash == hash
        {
            // 内容没变（只是 mtime 动过）：刷新元数据就够了
            conn.execute(
                "UPDATE items_fts SET mtime = ?, size = ? WHERE file = ?",
                params![mtime, size, file],
            )?;
            continue;
        }
        upsert(conn, &parse_item(&raw)?, file, mtime, size, hash)?;
    }

    let mut stale = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, file FROM items_fts")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let file: String = row.get(1)?;
            if !present.contains(&file) {
                stale.push(row.get::<_, String>(0)?);
            }
        }
    }
    for id in stale {
        conn.execute("DELETE FROM items_fts WHERE id = ?", [&id])?;
    }
    Ok(())
}

/// 写入 / 覆盖一条索引行（FTS5 虚拟表没有 UPSERT，先删后插）。
fn upsert(
    conn: &Connection,
    item: &Item,
    file: &str,
    mtime: i64,
    size: i64,
    hash: i64,
) -> Result<()> {
    conn.execute("DELETE FROM items_fts WHERE id = ?", [&item.id])?;
    conn.execute(
        "INSERT INTO items_fts
           (id, file, mtime, size, hash, title, body, type, status, space, project, priority, tags, due, created, data)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            item.id,
            file,
            mtime,
            size,
            hash,
            item.title,
            item.body,
            wire(item.item_type),
            wire(item.status),
            wire(item.space),
            item.project,
            item.priority.map(wire),
            tags_blob(&item.tags),
            item.due,
            item.created,
            serde_json::to_string(item)?,
        ],
    )?;
    Ok(())
}

/// 组装 SQL：各条件一律 AND；文本 3 字符以上走 FTS5 trigram 匹配，更短的查询回退
/// 标题 + 正文的子串扫描（trigram 对 <3 字符的查询天然不命中）。
fn build_sql(query: &SearchQuery) -> (String, Vec<Value>) {
    let mut sql = String::from("SELECT data FROM items_fts WHERE 1 = 1");
    let mut params = Vec::new();

    if let Some(text) = trimmed(query.text.as_deref()) {
        if text.chars().count() >= 3 {
            sql.push_str(" AND items_fts MATCH ?");
            params.push(Value::from(match_expression(text)));
        } else {
            sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\')");
            let pattern = Value::from(format!("%{}%", escape_like(text)));
            params.push(pattern.clone());
            params.push(pattern);
        }
    }

    in_list(
        &mut sql,
        &mut params,
        "type",
        query.types.iter().map(|kind| wire(*kind)),
    );
    in_list(
        &mut sql,
        &mut params,
        "status",
        query.statuses.iter().map(|status| wire(*status)),
    );
    in_list(
        &mut sql,
        &mut params,
        "priority",
        query.priority.iter().map(|priority| wire(*priority)),
    );
    for tag in &query.tags {
        // 索引里的标签串形如 "\n报表\n后端\n"：instr 命中即「含该标签」，也不会子串误命中
        sql.push_str(" AND instr(tags, ?) > 0");
        params.push(Value::from(format!("\n{tag}\n")));
    }
    if let Some(project) = trimmed(query.project.as_deref()) {
        sql.push_str(" AND project = ?");
        params.push(Value::from(project.to_string()));
    }
    if let Some(space) = query.space {
        sql.push_str(" AND space = ?");
        params.push(Value::from(wire(space)));
    }
    if let Some(due) = trimmed(query.due_before.as_deref()) {
        sql.push_str(" AND due <= ?");
        params.push(Value::from(due.to_string()));
    }
    if let Some(due) = trimmed(query.due_after.as_deref()) {
        sql.push_str(" AND due >= ?");
        params.push(Value::from(due.to_string()));
    }

    sql.push_str(" ORDER BY created DESC, id DESC");
    (sql, params)
}

/// `AND 列 IN (?, ?, …)`；没有取值时不加条件（多值 = 任一命中）。
fn in_list(
    sql: &mut String,
    params: &mut Vec<Value>,
    column: &str,
    values: impl Iterator<Item = String>,
) {
    let values: Vec<String> = values.collect();
    if values.is_empty() {
        return;
    }
    let placeholders = vec!["?"; values.len()].join(", ");
    sql.push_str(&format!(" AND {column} IN ({placeholders})"));
    params.extend(values.into_iter().map(Value::from));
}

/// trim 后为空的文本视为「没填」。
fn trimmed(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// FTS5 查询串：每个词包成双引号短语、内部引号翻倍——用户输入永远不会造成语法错误。
fn match_expression(text: &str) -> String {
    text.split_whitespace()
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// LIKE 通配符转义（配合 `ESCAPE '\'`）。
fn escape_like(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_")
}

/// 枚举 → 索引列的取值：与 frontmatter 共用同一套 serde 命名，避免两处映射漂移。
/// 这些枚举都是单元变体，序列化必然得到字符串，失败即代码错误。
fn wire<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value)
        .expect("单元枚举序列化为字符串")
        .trim_matches('"')
        .to_string()
}

/// 标签在索引里存成 `"\n标签1\n标签2\n"`。
fn tags_blob(tags: &[String]) -> String {
    let mut blob = String::from("\n");
    for tag in tags {
        blob.push_str(tag);
        blob.push('\n');
    }
    blob
}

/// 内容哈希（FNV-1a 64）：只用来判断内容真变了没有，不做安全用途。
fn fnv1a(bytes: &[u8]) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash as i64
}

/// 文件 mtime 取纳秒；取不到（读不到或 1970 之前的怪时间）记 0，交给哈希兜底。
fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|since| since.as_nanos() as i64)
        .unwrap_or(0)
}
