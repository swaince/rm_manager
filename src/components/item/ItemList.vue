<script setup lang="ts">
import { computed, ref } from 'vue'
import { useLocalStorage } from '@vueuse/core'
import { DataTable } from '@/components/data-table'
import type { DataTableColumn, DataTableRow } from '@/components/data-table'
import { MultiSelect, SelectField } from '@/components/form'
import ItemEditDialog from './ItemEditDialog.vue'
import {
  ALLOWED_STATUSES,
  ITEM_TYPE_LABELS,
  PRIORITIES,
  SPACE_LABELS,
  STATUS_LABELS,
  setStatus,
  updateItem,
  type Item,
  type ItemPriority,
  type ItemStatus,
  type Progress,
} from '@/apis/items'

const props = defineProps<{ items: Item[]; progress: Record<string, Progress> }>()
const emit = defineEmits<{ updated: [item: Item] }>()

/** 全部可选列；标题 / 状态 / 优先级 / 操作有行内交互，其余列走 DataTable 默认渲染 */
const ALL_COLUMNS: DataTableColumn[] = [
  { key: 'title', label: '标题', width: '14rem' },
  { key: 'id', label: 'ID' },
  { key: 'type', label: '类型' },
  { key: 'status', label: '状态', width: '11rem' },
  { key: 'priority', label: '优先级', width: '7rem' },
  { key: 'space', label: '空间' },
  { key: 'project', label: '项目' },
  { key: 'tags', label: '标签' },
  { key: 'due', label: '目标日期' },
  { key: 'created', label: '创建日期' },
  { key: 'actions', label: '操作' },
]
const DEFAULT_COLUMN_KEYS = [
  'title',
  'type',
  'status',
  'priority',
  'space',
  'due',
  'created',
  'actions',
]

const SORT_OPTIONS = [
  { value: 'created-desc', label: '创建时间：新→旧' },
  { value: 'created-asc', label: '创建时间：旧→新' },
  { value: 'priority', label: '优先级：高→低' },
  { value: 'title', label: '标题：拼音序' },
]

/** 排序与显示列是本机偏好，只进 localStorage（不进 vault、不进 git） */
const sort = useLocalStorage<string>('rm-manager:item-sort', 'created-desc')
const visibleKeys = useLocalStorage<string[]>('rm-manager:item-columns', DEFAULT_COLUMN_KEYS)
const columnOptions = ALL_COLUMNS.map((column) => ({ value: column.key, label: column.label }))

/** 一列都不勾时表格会空白，退回默认列 */
const columns = computed(() => {
  const keys = visibleKeys.value.length ? visibleKeys.value : DEFAULT_COLUMN_KEYS
  return ALL_COLUMNS.filter((column) => keys.includes(column.key))
})

/** 未设优先级的排最后 */
function priorityRank(item: Item) {
  return item.priority ? PRIORITIES.indexOf(item.priority) : PRIORITIES.length
}

const comparators: Record<string, (a: Item, b: Item) => number> = {
  'created-desc': (a, b) => b.created.localeCompare(a.created),
  'created-asc': (a, b) => a.created.localeCompare(b.created),
  priority: (a, b) => priorityRank(a) - priorityRank(b),
  title: (a, b) => a.title.localeCompare(b.title, 'zh-Hans-CN'),
}

/** 行值：展示字段给默认渲染用；item 留给行内编辑取回原始条目 */
const rows = computed<DataTableRow[]>(() =>
  [...props.items].sort(comparators[sort.value] ?? comparators['created-desc']).map((item) => ({
    id: item.id,
    item,
    title: item.title,
    type: ITEM_TYPE_LABELS[item.type],
    status: item.status,
    priority: item.priority ?? '',
    space: SPACE_LABELS[item.space],
    project: item.project ?? '',
    tags: item.tags.join('，'),
    due: item.due ?? '',
    created: item.created.slice(0, 10),
  })),
)

function itemOf(row: DataTableRow): Item {
  return row.item as Item
}

/** 行内状态只列该类型允许的状态；blocked / dropped 走编辑里的旁路动作，故为 disabled */
function statusOptions(item: Item) {
  return ALLOWED_STATUSES[item.type].map((value) => ({
    value,
    label: STATUS_LABELS[value],
    disabled: value === 'blocked' || value === 'dropped',
  }))
}

const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))

const errorMessage = ref('')

/** 行内改动的公共落盘：成功回传最新条目，失败在工具条上说明原因 */
async function save(action: () => Promise<Item>) {
  errorMessage.value = ''
  try {
    emit('updated', await action())
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

async function changeStatus(item: Item, value: string) {
  const next = value as ItemStatus
  if (!next || next === item.status) return

  await save(() => setStatus(item.id, next))
}

/** 改优先级：用该条目当前字段拼完整编辑载荷（核心 update_item 收全量，status / 旁路字段保持原值） */
async function changePriority(item: Item, value: string) {
  const priority = (value || null) as ItemPriority | null
  if (priority === item.priority) return

  await save(() =>
    updateItem(item.id, {
      title: item.title,
      type: item.type,
      space: item.space,
      project: item.project,
      priority,
      tags: item.tags,
      due: item.due,
      body: item.body,
    }),
  )
}

function bypassReason(item: Item) {
  return item.status === 'blocked' ? item.blocked_reason : item.dropped_reason
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex flex-wrap items-center gap-2">
      <SelectField
        :model-value="sort"
        :options="SORT_OPTIONS"
        :clearable="false"
        placeholder="排序"
        class="w-48"
        @update:model-value="(value) => (sort = value)"
      />
      <MultiSelect
        v-model="visibleKeys"
        :options="columnOptions"
        placeholder="显示列"
        clearable
        class="w-80"
      />
      <span class="text-muted-foreground text-xs">显示 {{ columns.length }} 列</span>
    </div>

    <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>

    <DataTable :columns="columns" :rows="rows" height="calc(100vh - 26rem)">
      <template #cell-title="{ row }">
        <div class="flex items-center gap-2">
          <span class="truncate font-medium" :title="itemOf(row).title">
            {{ itemOf(row).title }}
          </span>
          <span
            v-if="progress[itemOf(row).id]"
            class="text-muted-foreground inline-flex items-center rounded-md border px-1.5 py-0.5 text-xs font-medium tabular-nums"
          >
            {{ progress[itemOf(row).id].done }}/{{ progress[itemOf(row).id].total }}
          </span>
        </div>
      </template>

      <template #cell-status="{ row }">
        <div class="flex flex-col gap-1">
          <SelectField
            :model-value="itemOf(row).status"
            :options="statusOptions(itemOf(row))"
            :clearable="false"
            @update:model-value="(value) => changeStatus(itemOf(row), value)"
          />
          <span
            v-if="bypassReason(itemOf(row))"
            class="text-muted-foreground truncate text-xs"
            :title="bypassReason(itemOf(row)) ?? ''"
          >
            {{ bypassReason(itemOf(row)) }}
          </span>
        </div>
      </template>

      <template #cell-priority="{ row }">
        <SelectField
          :model-value="itemOf(row).priority ?? ''"
          :options="priorityOptions"
          placeholder="未设置"
          @update:model-value="(value) => changePriority(itemOf(row), value)"
        />
      </template>

      <template #cell-actions="{ row }">
        <ItemEditDialog :item="itemOf(row)" @updated="(item) => emit('updated', item)" />
      </template>
    </DataTable>
  </div>
</template>
