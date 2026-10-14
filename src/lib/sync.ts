import type { SyncFailure, SyncStatus } from '@/apis/sync'

/** 同步状态广播事件名（Rust 侧定时任务与命令都发它，见 src-tauri/src/sync.rs） */
export const SYNC_STATUS = 'sync-status'

/** 角标文案：状态 + 待推 / 待拉条数 */
export function syncStateText(status: SyncStatus): string {
  switch (status.state) {
    case 'not_configured':
      return '未配置远端'
    case 'synced':
      return '已同步'
    case 'ahead':
      return `待推送 ${status.ahead} 条`
    case 'behind':
      return `待拉取 ${status.behind} 条`
    case 'diverged':
      return `待推 ${status.ahead} · 待拉 ${status.behind}`
    case 'conflict':
      return '有冲突待解决'
    case 'error':
      return status.last_error?.kind === 'auth' ? '认证失败' : '同步失败'
  }
}

/** 角标圆点颜色：正常 / 待处理 / 出错 / 未配置 */
export function syncStateDotClass(status: SyncStatus): string {
  switch (status.state) {
    case 'synced':
      return 'bg-emerald-500'
    case 'ahead':
    case 'behind':
    case 'diverged':
      return 'bg-amber-500'
    case 'conflict':
    case 'error':
      return 'bg-destructive'
    case 'not_configured':
      return 'bg-muted-foreground'
  }
}

/** 失败的可操作提示：每类给「下一步做什么」（FR-9.1 的失败分类提示） */
export function syncFailureHint(failure: SyncFailure): string {
  switch (failure.kind) {
    case 'network':
      return '网络不可达：检查网络 / 代理后再试；定时同步会自动重试。'
    case 'auth':
      return '认证失败：重新填写访问令牌（PAT）并保存；Gitee 还需填写用户名。'
    case 'conflict':
      return '两端改了同一处：逐条选择保留双方 / 本地 / 远端，然后继续同步。'
  }
}
