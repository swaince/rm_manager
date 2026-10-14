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

/** 条目：属性来自文件 frontmatter，body 是 Markdown 正文 */
export interface Item {
  id: string
  type: ItemType
  title: string
  status: ItemStatus
  space: Space
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

export const ITEM_TYPES: readonly ItemType[] = [
  'idea',
  'requirement',
  'bug',
  'spike',
  'chore',
  'session',
]

export const SPACES: readonly Space[] = ['work', 'personal']

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
