<script setup lang="ts">
import { computed, ref } from 'vue'
import { Card, CardContent } from '@/components/ui/card'
import {
  ALLOWED_STATUSES,
  ITEM_TYPE_LABELS,
  STATUSES,
  STATUS_LABELS,
  setStatus,
  type Item,
  type ItemStatus,
} from '@/apis/items'

const props = defineProps<{ items: Item[] }>()
const emit = defineEmits<{ updated: [item: Item] }>()

/** 旁路状态不许直接拖入（必须填原因），固定收尾两列 */
const BYPASS_STATUSES: readonly ItemStatus[] = ['blocked', 'dropped']

/** 看板列：主流程 + 类型专属状态（保留 / 进行中 / 已收敛），blocked / dropped 收尾 */
const BOARD_STATUSES: readonly ItemStatus[] = [
  ...STATUSES.filter((status) => !BYPASS_STATUSES.includes(status)),
  'kept',
  'active',
  'concluded',
  ...BYPASS_STATUSES,
]

const dragging = ref<Item | null>(null)
const hoverStatus = ref<ItemStatus | null>(null)
const errorMessage = ref('')

const columns = computed(() =>
  BOARD_STATUSES.map((status) => ({
    status,
    items: props.items.filter((item) => item.status === status),
  })),
)

/** 可放下：非旁路状态、目标状态在该条目类型允许集合内、且不是原地不动 */
function canDrop(status: ItemStatus) {
  const item = dragging.value
  return (
    !!item &&
    !BYPASS_STATUSES.includes(status) &&
    status !== item.status &&
    ALLOWED_STATUSES[item.type].includes(status)
  )
}

function dragHint(status: ItemStatus) {
  return BYPASS_STATUSES.includes(status)
    ? '阻塞 / 放弃需填写原因，请用编辑里的旁路动作'
    : '该类型的允许状态不含此列'
}

function onDragStart(event: DragEvent, item: Item) {
  dragging.value = item
  errorMessage.value = ''
  event.dataTransfer?.setData('text/plain', item.id)
  if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move'
}

/** dragover 里 preventDefault 才算「允许放下」；不调用即拖不进去 */
function onDragOver(event: DragEvent, status: ItemStatus) {
  hoverStatus.value = status
  if (canDrop(status)) {
    event.preventDefault()
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'
  }
}

async function onDrop(event: DragEvent, status: ItemStatus) {
  hoverStatus.value = null
  if (!canDrop(status)) return
  event.preventDefault()
  const item = dragging.value
  dragging.value = null
  if (!item) return

  errorMessage.value = ''
  try {
    emit('updated', await setStatus(item.id, status))
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function resetDrag() {
  dragging.value = null
  hoverStatus.value = null
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <p class="text-muted-foreground text-xs">
      拖动卡片到其他列即改状态；阻塞 / 放弃需在编辑里填写原因，不能直接拖入。
    </p>
    <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>

    <div class="flex items-start gap-3 overflow-x-auto pb-2">
      <section
        v-for="column in columns"
        :key="column.status"
        class="flex min-h-24 w-56 shrink-0 flex-col gap-2 rounded-lg border p-2"
        :class="[
          hoverStatus === column.status && canDrop(column.status) && 'border-primary bg-muted/50',
          hoverStatus === column.status &&
            dragging &&
            !canDrop(column.status) &&
            'border-destructive',
        ]"
        @dragover="onDragOver($event, column.status)"
        @drop="onDrop($event, column.status)"
      >
        <header class="flex items-center justify-between gap-2 px-1">
          <span class="text-sm font-medium">{{ STATUS_LABELS[column.status] }}</span>
          <span class="text-muted-foreground text-xs tabular-nums">{{ column.items.length }}</span>
        </header>

        <p
          v-if="hoverStatus === column.status && dragging && !canDrop(column.status)"
          class="text-destructive px-1 text-xs"
        >
          {{ dragHint(column.status) }}
        </p>

        <ul class="flex flex-col gap-2">
          <li
            v-for="item in column.items"
            :key="item.id"
            draggable="true"
            class="cursor-grab active:cursor-grabbing"
            @dragstart="onDragStart($event, item)"
            @dragend="resetDrag"
          >
            <Card size="sm" class="gap-1 py-2">
              <CardContent class="flex flex-col gap-1 px-3">
                <span class="truncate text-sm font-medium" :title="item.title">{{
                  item.title
                }}</span>
                <span class="text-muted-foreground truncate text-xs">
                  {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} ·
                  {{ item.priority ?? '未设优先级' }}
                </span>
              </CardContent>
            </Card>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>
