<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { DatePicker } from '@/components/date-time'
import {
  ITEM_TYPE_LABELS,
  STATUS_LABELS,
  listItems,
  scheduleItem,
  setFocus,
  type Item,
} from '@/apis/items'
import { addDays, formatDate, mondayOf } from '@/lib/date'

/** WIP 软上限：doing 数超过它只提醒、不阻止开新 */
const WIP_LIMIT = 3
/** 今日焦点建议区间（只提示、不强制） */
const FOCUS_MIN = 1
const FOCUS_MAX = 3

const items = ref<Item[]>([])
const loading = ref(true)
const errorMessage = ref('')
const busyId = ref('')

const today = formatDate(new Date())
const thisMonday = mondayOf(today)
const nextMonday = addDays(thisMonday, 7)

/** 排序：优先级 P0 最前（未设最后）→ 目标日期 → 创建时间 */
const priorityRank: Record<string, number> = { P0: 0, P1: 1, P2: 2, P3: 3 }
function byPriority(a: Item, b: Item): number {
  const rank = (item: Item) => (item.priority ? priorityRank[item.priority] : 9)
  return (
    rank(a) - rank(b) ||
    (a.due ?? '9999').localeCompare(b.due ?? '9999') ||
    a.created.localeCompare(b.created)
  )
}

/** 需求池 = 已整理（非 inbox）且未排期（无周排期、无目标日期）的条目 */
const pool = computed(() =>
  items.value
    .filter(
      (item) =>
        !item.scheduled_week &&
        !item.due &&
        item.status !== 'inbox' &&
        item.status !== 'done' &&
        item.status !== 'dropped',
    )
    .sort(byPriority),
)

/** 周列：本周、下周，再加其他有排期条目的周（按周一日期由近到远） */
const weeks = computed(() => {
  const others = [
    ...new Set(
      items.value
        .map((item) => item.scheduled_week)
        .filter((week): week is string => !!week && week !== thisMonday && week !== nextMonday),
    ),
  ].sort()
  return [thisMonday, nextMonday, ...others]
})

function weekLabel(monday: string): string {
  if (monday === thisMonday) return '本周'
  if (monday === nextMonday) return '下周'
  return `${monday.slice(5)} 那周`
}

function weekItems(monday: string): Item[] {
  return items.value.filter((item) => item.scheduled_week === monday).sort(byPriority)
}

/** 今日焦点 = focus_on 等于今天的条目 */
const focusItems = computed(() => items.value.filter((item) => item.focus_on === today))

/** 候选：ready / scheduled；已选中但状态走远的也留着，方便移出 */
const focusCandidates = computed(() =>
  items.value
    .filter(
      (item) => item.status === 'ready' || item.status === 'scheduled' || item.focus_on === today,
    )
    .sort(byPriority),
)

const doingCount = computed(() => items.value.filter((item) => item.status === 'doing').length)

const focusHint = computed(() =>
  focusItems.value.length > FOCUS_MAX
    ? `已选 ${focusItems.value.length} 条，超过建议上限 ${FOCUS_MAX} 条（仅提示，不限制）`
    : `建议每天 ${FOCUS_MIN}–${FOCUS_MAX} 条；已选 ${focusItems.value.length} 条`,
)

async function load() {
  loading.value = true
  errorMessage.value = ''
  try {
    items.value = await listItems()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

/** 动作返回最新条目，就地替换避免整表重载闪烁 */
async function run(item: Item, action: () => Promise<Item>) {
  busyId.value = item.id
  errorMessage.value = ''
  try {
    const updated = await action()
    const index = items.value.findIndex((candidate) => candidate.id === updated.id)
    if (index !== -1) items.value[index] = updated
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    busyId.value = ''
  }
}

function schedule(item: Item, week: string | null) {
  void run(item, () => scheduleItem(item.id, week))
}

/** 指定周：所选日期归到它所在周的周一 */
function schedulePicked(item: Item, date: string) {
  if (date) schedule(item, mondayOf(date))
}

function toggleFocus(item: Item) {
  void run(item, () => setFocus(item.id, item.focus_on === today ? null : today))
}

onMounted(load)
</script>

<template>
  <main class="mx-auto flex min-h-screen w-full max-w-5xl flex-col gap-6 p-8">
    <header>
      <h1 class="text-xl font-semibold">规划</h1>
      <p class="text-muted-foreground text-sm">
        需求池 → 排入某一周 → 每天挑 1–3 条今日焦点；排期与焦点写入条目 frontmatter。
      </p>
    </header>

    <p
      v-if="doingCount > WIP_LIMIT"
      class="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm"
      role="status"
    >
      WIP 提醒：进行中（doing）{{ doingCount }} 条，超过软上限
      {{ WIP_LIMIT }} 条——只提醒，不阻止开新。
    </p>

    <p v-if="loading" class="text-muted-foreground text-sm">加载中…</p>

    <div v-else-if="errorMessage" class="space-y-3">
      <p class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>
      <Button variant="outline" @click="load">重试</Button>
    </div>

    <template v-else>
      <Card aria-label="今日焦点">
        <CardHeader>
          <CardTitle class="flex items-baseline justify-between gap-2">
            <span>今日焦点</span>
            <span class="text-muted-foreground text-xs font-normal">{{ today }}</span>
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <p
            class="text-xs"
            :class="focusItems.length > FOCUS_MAX ? 'text-destructive' : 'text-muted-foreground'"
          >
            {{ focusHint }}
          </p>

          <p v-if="!focusCandidates.length" class="text-muted-foreground text-sm">
            没有候选条目：先把条目推进到「就绪」或「已排期」。
          </p>
          <ul v-else class="space-y-2">
            <li
              v-for="item in focusCandidates"
              :key="item.id"
              class="flex flex-wrap items-center justify-between gap-3"
            >
              <div class="flex min-w-0 flex-col gap-1">
                <span class="flex items-center gap-2">
                  <span class="truncate text-sm font-medium">{{ item.title }}</span>
                  <Badge v-if="item.focus_on === today" variant="default">今日</Badge>
                </span>
                <span class="text-muted-foreground text-xs">
                  {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} ·
                  {{ STATUS_LABELS[item.status] }}
                  <template v-if="item.scheduled_week">
                    · 排期 {{ item.scheduled_week }} 那周</template
                  >
                </span>
              </div>
              <Button
                size="sm"
                :variant="item.focus_on === today ? 'secondary' : 'outline'"
                :disabled="busyId === item.id"
                @click="toggleFocus(item)"
              >
                {{ item.focus_on === today ? '移出今日' : '加入今日' }}
              </Button>
            </li>
          </ul>
        </CardContent>
      </Card>

      <Card aria-label="需求池">
        <CardHeader>
          <CardTitle class="flex items-baseline justify-between gap-2">
            <span>需求池</span>
            <span class="text-muted-foreground text-xs font-normal">{{ pool.length }} 条</span>
          </CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <p class="text-muted-foreground text-xs">
            已整理、尚未排期的条目，按优先级 → 目标日期 →
            创建时间排序；排入某周或设目标日期后离开需求池。
          </p>

          <Empty v-if="!pool.length">
            <EmptyHeader>
              <EmptyTitle>需求池是空的</EmptyTitle>
              <EmptyDescription
                >整理好条目后（推进到就绪等状态且未排期）会出现在这里。</EmptyDescription
              >
            </EmptyHeader>
          </Empty>

          <ul v-else class="space-y-2">
            <li
              v-for="item in pool"
              :key="item.id"
              class="flex flex-wrap items-center justify-between gap-3"
            >
              <div class="flex min-w-0 flex-col gap-1">
                <span class="flex items-center gap-2">
                  <span class="truncate text-sm font-medium">{{ item.title }}</span>
                  <Badge v-if="item.priority" variant="outline">{{ item.priority }}</Badge>
                </span>
                <span class="text-muted-foreground text-xs">
                  {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} ·
                  {{ STATUS_LABELS[item.status] }}
                </span>
              </div>
              <div class="flex flex-wrap items-center gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  :disabled="busyId === item.id"
                  @click="schedule(item, thisMonday)"
                >
                  排入本周
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  :disabled="busyId === item.id"
                  @click="schedule(item, nextMonday)"
                >
                  排入下周
                </Button>
                <div class="w-28">
                  <DatePicker
                    :model-value="''"
                    placeholder="指定周"
                    aria-label="指定周"
                    @update:model-value="(value) => schedulePicked(item, value)"
                  />
                </div>
              </div>
            </li>
          </ul>
        </CardContent>
      </Card>

      <section aria-label="周视图" class="grid gap-4 lg:grid-cols-3">
        <Card v-for="monday in weeks" :key="monday">
          <CardHeader>
            <CardTitle class="flex items-baseline justify-between gap-2 text-base">
              <span>{{ weekLabel(monday) }}</span>
              <span class="text-muted-foreground text-xs font-normal">{{ monday }} 起</span>
            </CardTitle>
          </CardHeader>
          <CardContent class="space-y-2">
            <p v-if="!weekItems(monday).length" class="text-muted-foreground text-sm">（无）</p>
            <ul v-else class="space-y-2">
              <li
                v-for="item in weekItems(monday)"
                :key="item.id"
                class="flex flex-wrap items-start justify-between gap-2"
              >
                <div class="flex min-w-0 flex-col gap-1">
                  <span class="flex items-center gap-2">
                    <span class="truncate text-sm font-medium">{{ item.title }}</span>
                    <Badge v-if="item.priority" variant="outline">{{ item.priority }}</Badge>
                  </span>
                  <span class="text-muted-foreground text-xs">
                    {{ STATUS_LABELS[item.status] }}
                    <template v-if="item.due"> · 目标 {{ item.due }}</template>
                  </span>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  :disabled="busyId === item.id"
                  @click="schedule(item, null)"
                >
                  移出
                </Button>
              </li>
            </ul>
          </CardContent>
        </Card>
      </section>
    </template>
  </main>
</template>
