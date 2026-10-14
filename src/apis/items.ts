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
  created: string
  body: string
}

/** 新建条目载荷；id / status / created 由核心生成 */
export interface ItemDraft {
  title: string
  type: ItemType
  space: Space
  body: string
}

/** 编辑载荷 = 完整可编辑状态（清空 = null / 空数组）；id / status / created 不可编辑 */
export interface ItemEdit {
  title: string
  type: ItemType
  space: Space
  project: string | null
  priority: ItemPriority | null
  tags: string[]
  due: string | null
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
