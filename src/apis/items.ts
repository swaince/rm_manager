import { invoke } from '@tauri-apps/api/core'

export type ItemType = 'idea' | 'requirement' | 'bug' | 'spike' | 'chore' | 'session'
export type Space = 'work' | 'personal'
export type ItemStatus =
  | 'inbox'
  | 'clarifying'
  | 'refining'
  | 'ready'
  | 'scheduled'
  | 'doing'
  | 'blocked'
  | 'review'
  | 'done'
  | 'dropped'
  | 'kept'
  | 'active'
  | 'concluded'

export type ItemPriority = 'P0' | 'P1' | 'P2' | 'P3'

/** 估算三档预算；人日 / 点数只是展示口径（见 lib/estimate.ts），不落盘 */
export type ItemEstimate = 'small' | 'medium' | 'large'

/** 验收标准 checklist 的一项 */
export interface AcceptanceCriterion {
  text: string
  done: boolean
}

/** 澄清记录：answer 为空即待答复；source 是来源人 */
export interface Clarification {
  question: string
  answer: string | null
  source: string | null
}

/** 条目：属性来自文件 frontmatter，body 是 Markdown 正文；可选字段未设置为 null / 空数组 */
export interface Item {
  id: string
  type: ItemType
  title: string
  status: ItemStatus
  blocked_reason: string | null
  blocked_waiting_on: string | null
  blocked_from: ItemStatus | null
  dropped_reason: string | null
  space: Space
  project: string | null
  priority: ItemPriority | null
  tags: string[]
  due: string | null
  parent: string | null
  estimate: ItemEstimate | null
  acceptance: AcceptanceCriterion[]
  clarifications: Clarification[]
  /** 就绪检查豁免理由；未豁免为 null */
  dor_waived: string | null
  created: string
  body: string
}

/** 父项聚合进度：total = 未 dropped 的直接子项数，done = 其中已完成的数量 */
export interface Progress {
  done: number
  total: number
}

/** 新建条目载荷；id / status / created 由核心生成 */
export interface ItemDraft {
  title: string
  type: ItemType
  space: Space
  body: string
}

/** 编辑载荷 = 完整可编辑状态（清空 = null / 空数组）；id / status / created / dor_waived 不可编辑 */
export interface ItemEdit {
  title: string
  type: ItemType
  space: Space
  project: string | null
  priority: ItemPriority | null
  tags: string[]
  due: string | null
  estimate: ItemEstimate | null
  acceptance: AcceptanceCriterion[]
  clarifications: Clarification[]
  body: string
}

export const ITEM_TYPES: readonly ItemType[] = [
  'idea',
  'requirement',
  'bug',
  'spike',
  'chore',
  'session',
]

export const SPACES: readonly Space[] = ['work', 'personal']

export const PRIORITIES: readonly ItemPriority[] = ['P0', 'P1', 'P2', 'P3']

/** 估算三档，顺序 = 小 → 大 */
export const ESTIMATES: readonly ItemEstimate[] = ['small', 'medium', 'large']

export const STATUSES: readonly ItemStatus[] = [
  'inbox',
  'clarifying',
  'refining',
  'ready',
  'scheduled',
  'doing',
  'blocked',
  'review',
  'done',
  'dropped',
]

/** 各类型允许的状态集合（与 Rust allowed_statuses 一致）；blocked / dropped 走独立动作 */
export const ALLOWED_STATUSES: Record<ItemType, readonly ItemStatus[]> = {
  idea: ['inbox', 'kept', 'blocked', 'dropped'],
  requirement: [
    'inbox',
    'clarifying',
    'refining',
    'ready',
    'scheduled',
    'doing',
    'review',
    'done',
    'blocked',
    'dropped',
  ],
  bug: ['inbox', 'ready', 'scheduled', 'doing', 'review', 'done', 'blocked', 'dropped'],
  chore: ['inbox', 'ready', 'scheduled', 'doing', 'done', 'blocked', 'dropped'],
  spike: ['inbox', 'doing', 'done', 'blocked', 'dropped'],
  session: ['active', 'concluded', 'blocked', 'dropped'],
}

export const ITEM_TYPE_LABELS: Record<ItemType, string> = {
  idea: '想法',
  requirement: '需求',
  bug: 'Bug',
  spike: '预研',
  chore: '杂务',
  session: '头脑风暴会话',
}

export const SPACE_LABELS: Record<Space, string> = {
  work: '工作',
  personal: '个人',
}

export const STATUS_LABELS: Record<ItemStatus, string> = {
  inbox: '收件箱',
  clarifying: '澄清中',
  refining: '细化中',
  ready: '就绪',
  scheduled: '已排期',
  doing: '进行中',
  blocked: '阻塞',
  review: '待验收',
  done: '已完成',
  dropped: '已放弃',
  kept: '保留',
  active: '进行中',
  concluded: '已收敛',
}

export function listItems(): Promise<Item[]> {
  return invoke<Item[]>('list_items')
}

export function createItem(draft: ItemDraft): Promise<Item> {
  // 命令参数按 camelCase 传（Rust 侧是 item_type）
  return invoke<Item>('create_item', {
    title: draft.title,
    itemType: draft.type,
    space: draft.space,
    body: draft.body,
  })
}

export function updateItem(id: string, edit: ItemEdit): Promise<Item> {
  return invoke<Item>('update_item', { id, edit })
}

/** 搜索条件：空条件 = 不过滤；字段名与核心 SearchQuery、保存视图的 YAML 一致（snake_case） */
export interface SearchQuery {
  text: string | null
  types: ItemType[]
  statuses: ItemStatus[]
  tags: string[]
  priority: ItemPriority[]
  project: string | null
  space: Space | null
  /** 目标日期区间，含端点 */
  due_before: string | null
  due_after: string | null
}

/** 保存的视图：name 即文件名（views/<name>.yaml），query 是完整搜索条件 */
export interface SavedView {
  name: string
  query: SearchQuery
}

export function emptySearchQuery(): SearchQuery {
  return {
    text: null,
    types: [],
    statuses: [],
    tags: [],
    priority: [],
    project: null,
    space: null,
    due_before: null,
    due_after: null,
  }
}

export function searchItems(query: SearchQuery): Promise<Item[]> {
  return invoke<Item[]>('search_items', { query })
}

/** 全量重建索引（索引可随时重建，删了/坏了都能用它兜底） */
export function rebuildIndex(): Promise<void> {
  return invoke('rebuild_index')
}

export function listViews(): Promise<SavedView[]> {
  return invoke<SavedView[]>('list_views')
}

/** 保存视图；同名覆盖 */
export function saveView(name: string, query: SearchQuery): Promise<SavedView> {
  return invoke<SavedView>('save_view', { name, query })
}

export function deleteView(name: string): Promise<void> {
  return invoke('delete_view', { name })
}

export function setStatus(id: string, status: ItemStatus): Promise<Item> {
  return invoke<Item>('set_status', { id, status })
}

export function blockItem(id: string, reason: string, waitingOn: string): Promise<Item> {
  return invoke<Item>('block_item', { id, reason, waitingOn })
}

export function unblockItem(id: string): Promise<Item> {
  return invoke<Item>('unblock_item', { id })
}

export function dropItem(id: string, reason: string): Promise<Item> {
  return invoke<Item>('drop_item', { id, reason })
}

/** 豁免就绪检查（FR-4.3）：理由必填并写进 frontmatter 留痕；豁免后该需求可进 ready */
export function waiveDor(id: string, reason: string): Promise<Item> {
  return invoke<Item>('waive_dor', { id, reason })
}

/** 拆解：给子条目设置 / 清空父条目（null 清空），返回更新后的条目 */
export function setParent(id: string, parent: string | null): Promise<Item> {
  return invoke<Item>('set_parent', { id, parent })
}

export function childrenOf(id: string): Promise<Item[]> {
  return invoke<Item[]>('children_of', { id })
}

export function parentProgress(id: string): Promise<Progress> {
  return invoke<Progress>('parent_progress', { id })
}
