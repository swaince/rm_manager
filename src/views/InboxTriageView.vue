<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { SelectField, TextInput } from '@/components/form'
import ItemEditDialog from '@/components/item/ItemEditDialog.vue'
import { ITEMS_CHANGED } from '@/lib/capture'
import {
  emptySearchQuery,
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  PRIORITIES,
  searchItems,
  setStatus,
  SPACES,
  SPACE_LABELS,
  STATUS_LABELS,
  updateItem,
  type Item,
  type ItemEdit,
  type ItemPriority,
  type ItemStatus,
  type ItemType,
  type Space,
} from '@/apis/items'

/** 转类型并推进：各类型离开收件箱后的第一个实质状态（与核心 allowed_statuses 一致） */
const ADVANCE_STATUS: Record<ItemType, ItemStatus> = {
  idea: 'kept',
  requirement: 'clarifying',
  bug: 'ready',
  chore: 'ready',
  spike: 'doing',
  session: 'active',
}

const DAY_MS = 86_400_000

const items = ref<Item[]>([])
const checkedIds = ref<string[]>([])
const loading = ref(true)
const busy = ref(false)
const errorMessage = ref('')
const notice = ref('')

/** 批量动作条：每个动作一个控件，值空时按钮禁用 */
const typeValue = ref('')
const spaceValue = ref('')
const projectValue = ref('')
const tagsValue = ref('')
const priorityValue = ref('')
const advanceType = ref('')

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))
const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))

const selected = computed(() => items.value.filter((item) => checkedIds.value.includes(item.id)))
const canApply = computed(() => selected.value.length > 0 && !busy.value)
const allSelected = computed(
  () => items.value.length > 0 && checkedIds.value.length === items.value.length,
)

/** 清理提醒：最早一条已堆 N 天（FR-1.3） */
const oldestDays = computed(() => {
  if (!items.value.length) return 0
  const oldest = items.value.reduce((left, right) => (left.created <= right.created ? left : right))
  return Math.max(0, Math.floor((Date.now() - Date.parse(oldest.created)) / DAY_MS))
})

/** 「转类型并推进」的目标说明：转为 X → 落到 Y 状态 */
const advanceHint = computed(() => {
  const target = advanceType.value as ItemType | ''
  if (selected.value.length === 0 || !target) return ''
  return `${ITEM_TYPE_LABELS[target]} → ${STATUS_LABELS[ADVANCE_STATUS[target]]}`
})

async function load() {
  loading.value = true
  errorMessage.value = ''
  try {
    items.value = await searchItems({ ...emptySearchQuery(), statuses: ['inbox'] })
    // 整理掉的条目不再留在选择里
    checkedIds.value = checkedIds.value.filter((id) => items.value.some((item) => item.id === id))
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

let unlisten: (() => void) | undefined

onMounted(async () => {
  await load()
  unlisten = await listen(ITEMS_CHANGED, () => void load())
})

onUnmounted(() => unlisten?.())

function toggle(id: string) {
  checkedIds.value = checkedIds.value.includes(id)
    ? checkedIds.value.filter((candidate) => candidate !== id)
    : [...checkedIds.value, id]
}

function toggleAll() {
  checkedIds.value = allSelected.value ? [] : items.value.map((item) => item.id)
}

/** 单个条目的当前字段拼成 ItemEdit；批量整理 = 各自现有字段 + 覆盖项（不做跨条目复制） */
function toEdit(item: Item): ItemEdit {
  return {
    title: item.title,
    type: item.type,
    space: item.space,
    project: item.project,
    priority: item.priority,
    tags: item.tags,
    due: item.due,
    body: item.body,
  }
}

/** 批量：对选中条目逐个 updateItem（可再 setStatus 推进）；中途失败即停并报错，已落盘的保留 */
async function applyBulk(patch: (item: Item) => Partial<ItemEdit>, status?: ItemStatus) {
  const targets = selected.value
  if (!targets.length || busy.value) return
  busy.value = true
  notice.value = ''
  errorMessage.value = ''
  try {
    for (const item of targets) {
      await updateItem(item.id, { ...toEdit(item), ...patch(item) })
      if (status) await setStatus(item.id, status)
    }
    checkedIds.value = []
    await load()
    notice.value = `已整理 ${targets.length} 条`
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
    await load()
  } finally {
    busy.value = false
  }
}

function setType() {
  if (typeValue.value) void applyBulk(() => ({ type: typeValue.value as ItemType }))
}

function setSpace() {
  if (spaceValue.value) void applyBulk(() => ({ space: spaceValue.value as Space }))
}

function setProject() {
  const project = projectValue.value.trim()
  if (project) void applyBulk(() => ({ project }))
}

function setPriority() {
  if (priorityValue.value) void applyBulk(() => ({ priority: priorityValue.value as ItemPriority }))
}

/** 加标签：与各自现有标签并集去重 */
function addTags() {
  const tags = tagsValue.value
    .split(/[,，]/)
    .map((tag) => tag.trim())
    .filter(Boolean)
  if (tags.length) void applyBulk((item) => ({ tags: [...new Set([...item.tags, ...tags])] }))
}

/** 转类型并推进：转为目标类型后 setStatus 到该类型离开收件箱的第一个状态 */
function convertAndAdvance() {
  const target = advanceType.value as ItemType
  if (target) void applyBulk(() => ({ type: target }), ADVANCE_STATUS[target])
}
</script>

<template>
  <main class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-8 pt-4">
    <header class="flex flex-wrap items-baseline justify-between gap-2">
      <div>
        <h1 class="text-xl font-semibold">收件箱整理台</h1>
        <p class="text-muted-foreground text-sm">
          给 inbox 条目补全属性并推进状态；整理不搬动任何数据。
        </p>
      </div>
      <p v-if="oldestDays > 0" class="text-muted-foreground text-xs">
        最早一条已堆 {{ oldestDays }} 天
      </p>
    </header>

    <section
      v-if="items.length"
      class="flex flex-col gap-3 rounded-lg border p-3"
      aria-label="批量整理"
    >
      <div class="flex flex-wrap items-center gap-2">
        <Checkbox
          :model-value="allSelected"
          aria-label="全选收件箱条目"
          @update:model-value="toggleAll"
        />
        <span class="text-sm">已选 {{ selected.length }} / {{ items.length }} 条</span>
        <Button
          v-if="selected.length"
          variant="ghost"
          size="sm"
          :disabled="busy"
          @click="checkedIds = []"
        >
          清空选择
        </Button>
        <span v-if="notice" class="text-muted-foreground text-xs" role="status">{{ notice }}</span>
      </div>

      <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <div class="flex items-center gap-2">
          <SelectField
            :model-value="typeValue"
            :options="typeOptions"
            :clearable="false"
            placeholder="类型"
            class="w-32"
            aria-label="目标类型"
            @update:model-value="(value) => (typeValue = value)"
          />
          <Button variant="outline" size="sm" :disabled="!canApply || !typeValue" @click="setType">
            设类型
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <SelectField
            :model-value="spaceValue"
            :options="spaceOptions"
            :clearable="false"
            placeholder="空间"
            class="w-28"
            aria-label="目标空间"
            @update:model-value="(value) => (spaceValue = value)"
          />
          <Button
            variant="outline"
            size="sm"
            :disabled="!canApply || !spaceValue"
            @click="setSpace"
          >
            设空间
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <TextInput v-model="projectValue" placeholder="项目" class="w-40" aria-label="目标项目" />
          <Button
            variant="outline"
            size="sm"
            :disabled="!canApply || !projectValue.trim()"
            @click="setProject"
          >
            设项目
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <TextInput
            v-model="tagsValue"
            placeholder="标签，逗号分隔"
            class="w-44"
            aria-label="新增标签"
          />
          <Button variant="outline" size="sm" :disabled="!canApply" @click="addTags">
            加标签
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <SelectField
            :model-value="priorityValue"
            :options="priorityOptions"
            placeholder="优先级"
            class="w-28"
            aria-label="目标优先级"
            @update:model-value="(value) => (priorityValue = value)"
          />
          <Button
            variant="outline"
            size="sm"
            :disabled="!canApply || !priorityValue"
            @click="setPriority"
          >
            设优先级
          </Button>
        </div>

        <div class="flex items-center gap-2">
          <SelectField
            :model-value="advanceType"
            :options="typeOptions"
            :clearable="false"
            placeholder="转为"
            class="w-32"
            aria-label="转类型目标"
            @update:model-value="(value) => (advanceType = value)"
          />
          <Button
            variant="secondary"
            size="sm"
            :disabled="!canApply || !advanceType"
            @click="convertAndAdvance"
          >
            转类型并推进
          </Button>
        </div>
      </div>

      <p v-if="advanceHint" class="text-muted-foreground text-xs">推进目标：{{ advanceHint }}</p>
    </section>

    <p v-if="loading" class="text-muted-foreground text-sm">加载中…</p>

    <div v-else-if="errorMessage" class="space-y-3">
      <p class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>
      <Button variant="outline" @click="load">重试</Button>
    </div>

    <Card v-else-if="items.length === 0">
      <CardContent>
        <Empty>
          <EmptyHeader>
            <EmptyTitle>收件箱是空的</EmptyTitle>
            <EmptyDescription
              >按 Ctrl+Shift+Space 速记，或 Ctrl+Shift+V 收割剪贴板。</EmptyDescription
            >
          </EmptyHeader>
        </Empty>
      </CardContent>
    </Card>

    <ul v-else class="space-y-2">
      <li v-for="item in items" :key="item.id">
        <Card>
          <CardContent class="flex items-center gap-3">
            <Checkbox
              :model-value="checkedIds.includes(item.id)"
              :aria-label="`选择 ${item.title}`"
              @update:model-value="() => toggle(item.id)"
            />
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span class="font-medium">{{ item.title }}</span>
              <span class="text-muted-foreground text-xs">
                {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} · {{ SPACE_LABELS[item.space] }} ·
                {{ item.created.slice(0, 10) }}
              </span>
            </div>
            <ItemEditDialog :item="item" @updated="load" />
          </CardContent>
        </Card>
      </li>
    </ul>
  </main>
</template>
