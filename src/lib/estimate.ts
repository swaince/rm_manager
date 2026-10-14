import { useStorage } from '@vueuse/core'
import type { ItemEstimate } from '@/apis/items'

/** 估算展示口径（FR-4.5）：可切人日 / 点数；vault 里只存三档 */
export type EstimateUnit = 'days' | 'points'

export const ESTIMATE_UNITS: readonly EstimateUnit[] = ['days', 'points']

export const ESTIMATE_UNIT_LABELS: Record<EstimateUnit, string> = {
  days: '人日',
  points: '点数',
}

export const ESTIMATE_LABELS: Record<ItemEstimate, string> = {
  small: '小',
  medium: '中',
  large: '大',
}

/** 展示映射表：小 0.5 人日 / 1 点，中 2 人日 / 3 点，大 5 人日 / 8 点 */
const ESTIMATE_VALUES: Record<EstimateUnit, Record<ItemEstimate, string>> = {
  days: { small: '0.5 人日', medium: '2 人日', large: '5 人日' },
  points: { small: '1 点', medium: '3 点', large: '8 点' },
}

/** 展示单位存在本地设置里（不进 vault，不参与同步） */
export const estimateUnit = useStorage<EstimateUnit>('rm-manager.estimate-unit', 'days')

/** 估算的展示文本：档位 + 当前口径取值；未估算返回空串 */
export function estimateText(estimate: ItemEstimate | null): string {
  if (!estimate) return ''
  return `${ESTIMATE_LABELS[estimate]}（${ESTIMATE_VALUES[estimateUnit.value][estimate]}）`
}
