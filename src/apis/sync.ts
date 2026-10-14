import { invoke } from '@tauri-apps/api/core'

/** 同步状态（与 Rust SyncState 一致，serde snake_case） */
export type SyncState =
  'not_configured' | 'synced' | 'ahead' | 'behind' | 'diverged' | 'conflict' | 'error'

/** 失败分类：网络 / 认证 / 冲突，前端据此给可操作提示（FR-9.1） */
export type SyncErrorKind = 'network' | 'auth' | 'conflict'

export interface SyncFailure {
  kind: SyncErrorKind
  /** git 自己的话，原样展示便于排查 */
  message: string
}

/** 同步状态快照：ahead / behind 相对本地已拉取的远端；pulled / pushed 是本次操作的提交数 */
export interface SyncStatus {
  state: SyncState
  ahead: number
  behind: number
  pulled: number
  pushed: number
  last_error: SyncFailure | null
}

/** 一处冲突：local / remote 都是完整的条目文件文本（rebase 中 stage :3: / :2:） */
export interface ConflictEntry {
  /** vault 相对路径，resolveConflict 原样传回 */
  path: string
  item_id: string | null
  title: string | null
  local: string
  remote: string
}

export type ConflictResolution = 'use_remote' | 'use_local' | 'keep_both'

/** 同步状态（纯本地查询，不触网） */
export function syncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>('sync_status')
}

/** 手动同步一次：本地兜底 commit → fetch → pull --rebase → push */
export function syncNow(): Promise<SyncStatus> {
  return invoke<SyncStatus>('sync')
}

/** 连接远端并首次推送；last_error 非空表示首次推送失败（网络 / 认证），远端配置已生效 */
export function setRemote(url: string, user: string, token: string): Promise<SyncStatus> {
  return invoke<SyncStatus>('set_remote', { url, user, token })
}

/** 测试连接：只读探测（ls-remote），不改配置、不写凭据；null = 连接成功 */
export function testRemote(url: string, user: string, token: string): Promise<SyncFailure | null> {
  return invoke<SyncFailure | null>('test_remote', { url, user, token })
}

/** 待解决的冲突条目（rebase 中止时的现场） */
export function syncConflicts(): Promise<ConflictEntry[]> {
  return invoke<ConflictEntry[]>('sync_conflicts')
}

/** 三选一解决一处冲突（解决后要 continueSync 收尾） */
export function resolveConflict(path: string, resolution: ConflictResolution): Promise<void> {
  return invoke('resolve_conflict', { path, resolution })
}

/** 冲突全部解决后收尾：rebase --continue + push */
export function continueSync(): Promise<SyncStatus> {
  return invoke<SyncStatus>('continue_sync')
}
