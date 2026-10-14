<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { FormField, MultiSelect, SelectField, TextInput } from '@/components/form'
import { DatePicker } from '@/components/date-time'
import ItemCreateDialog from '@/components/item/ItemCreateDialog.vue'
import ItemEditDialog from '@/components/item/ItemEditDialog.vue'
import {
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  PRIORITIES,
  SPACES,
  SPACE_LABELS,
  STATUSES,
  STATUS_LABELS,
  deleteView,
  emptySearchQuery,
  listViews,
  parentProgress,
  rebuildIndex,
  saveView,
  searchItems,
  type Item,
  type ItemPriority,
  type ItemStatus,
  type ItemType,
  type Progress,
  type SavedView,
  type SearchQuery,
  type Space,
} from '@/apis/items'

/** 停止输入 300ms 后查询 */
const SEARCH_DELAY_MS = 300

const items = ref<Item[]>([])
const loading = ref(true)
const errorMessage = ref('')
const notice = ref('')
const filtering = ref(false)

/** 筛选条：各字段单独持有，查之前攒成 SearchQuery */
const text = ref('')
const types = ref<string[]>([])
const statuses = ref<string[]>([])
const tags = ref('')
const priority = ref<string[]>([])
const space = ref('')
const project = ref('')
const dueAfter = ref('')
const dueBefore = ref('')

const views = ref<SavedView[]>([])
const selectedView = ref('')
const saveOpen = ref(false)
const viewName = ref('')
const saveError = ref('')
const rebuilding = ref(false)

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const statusOptions = STATUSES.map((value) => ({ value, label: STATUS_LABELS[value] }))
const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))
const viewOptions = computed(() =>
  views.value.map((view) => ({ value: view.name, label: view.name })),
)

/** 筛选条 → 搜索条件；空值一律记 null / 空数组，等于「不过滤」。 */
function currentQuery(): SearchQuery {
  return {
    text: text.value.trim() || null,
    types: types.value as ItemType[],
    statuses: statuses.value as ItemStatus[],
    // 标签用中英文逗号分隔，与编辑弹窗一致
    tags: tags.value
      .split(/[,，]/)
      .map((tag) => tag.trim())
      .filter(Boolean),
    priority: priority.value as ItemPriority[],
    project: project.value.trim() || null,
    space: (space.value || null) as Space | null,
    due_before: dueBefore.value || null,
    due_after: dueAfter.value || null,
  }
}

/** 选中的视图 / 清空筛选，都把条件回填到筛选条 */
function applyQuery(query: SearchQuery) {
  text.value = query.text ?? ''
  types.value = [...query.types]
  statuses.value = [...query.statuses]
  tags.value = query.tags.join('，')
  priority.value = [...query.priority]
  space.value = query.space ?? ''
  project.value = query.project ?? ''
  dueAfter.value = query.due_after ?? ''
  dueBefore.value = query.due_before ?? ''
}

/** 父项进度徽标（完成 x/y）：只给「被别的条目当作父」的条目取一次聚合 */
const progress = ref<Record<string, Progress>>({})

async function refreshProgress() {
  const parentIds = [
    ...new Set(items.value.map((item) => item.parent).filter((id): id is string => !!id)),
  ]
  const entries = await Promise.all(
    parentIds.map(async (id) => [id, await parentProgress(id)] as const),
  )
  progress.value = Object.fromEntries(entries)
}

async function load() {
  const query = currentQuery()
  filtering.value = JSON.stringify(query) !== JSON.stringify(emptySearchQuery())
  loading.value = true
  errorMessage.value = ''
  try {
    items.value = await searchItems(query)
    await refreshProgress()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

const debouncedLoad = useDebounceFn(load, SEARCH_DELAY_MS)

watch([text, types, statuses, tags, priority, space, project, dueAfter, dueBefore], () => {
  // 手动改过条件就不再算「正在看某个视图」
  selectedView.value = ''
  notice.value = ''
  void debouncedLoad()
})

async function loadViews() {
  try {
    views.value = await listViews()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function applyView(name: string) {
  const view = views.value.find((candidate) => candidate.name === name)
  if (!view) return
  selectedView.value = name
  applyQuery(view.query)
  void load()
}

async function saveCurrentView() {
  saveError.value = ''
  try {
    const saved = await saveView(viewName.value, currentQuery())
    await loadViews()
    selectedView.value = saved.name
    viewName.value = ''
    saveOpen.value = false
    notice.value = `已保存视图「${saved.name}」`
  } catch (error) {
    saveError.value = error instanceof Error ? error.message : String(error)
  }
}

async function removeSelectedView() {
  const name = selectedView.value
  selectedView.value = ''
  try {
    await deleteView(name)
    await loadViews()
    notice.value = `已删除视图「${name}」`
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function clearFilters() {
  selectedView.value = ''
  applyQuery(emptySearchQuery())
  void load()
}

async function rebuild() {
  rebuilding.value = true
  errorMessage.value = ''
  try {
    await rebuildIndex()
    await load()
    notice.value = '索引已全量重建'
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    rebuilding.value = false
  }
}

onMounted(async () => {
  await Promise.all([load(), loadViews()])
})

/** 编辑对话框每次落盘都会回传最新条目，就地替换避免整表重载闪烁 */
function onUpdated(updated: Item) {
  const index = items.value.findIndex((item) => item.id === updated.id)
  if (index !== -1) items.value[index] = updated
  void refreshProgress()
}
</script>

<template>
  <main class="mx-auto flex min-h-screen w-full max-w-3xl flex-col gap-6 p-8">
    <header class="flex items-start justify-between gap-4">
      <div>
        <h1 class="text-xl font-semibold">条目</h1>
        <p class="text-muted-foreground text-sm">
          数据真源是 vault 里的 Markdown 文件，一条目一文件。
        </p>
      </div>
      <ItemCreateDialog @created="load" />
    </header>

    <section class="flex flex-col gap-3" aria-label="搜索与筛选">
      <TextInput v-model="text" type="search" clearable placeholder="搜索标题与正文，如：报表" />

      <div class="grid gap-3 sm:grid-cols-3">
        <MultiSelect v-model="types" :options="typeOptions" placeholder="类型" clearable />
        <MultiSelect v-model="statuses" :options="statusOptions" placeholder="状态" clearable />
        <MultiSelect v-model="priority" :options="priorityOptions" placeholder="优先级" clearable />
        <SelectField
          :model-value="space"
          :options="spaceOptions"
          placeholder="空间：全部"
          @update:model-value="(value) => (space = value)"
        />
        <TextInput v-model="project" clearable placeholder="项目（留空不筛）" />
        <TextInput v-model="tags" clearable placeholder="标签，逗号分隔" />
        <DatePicker v-model="dueAfter" clearable placeholder="截止日期从" aria-label="截止日期从" />
        <DatePicker
          v-model="dueBefore"
          clearable
          placeholder="截止日期到"
          aria-label="截止日期到"
        />
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <SelectField
          :model-value="selectedView"
          :options="viewOptions"
          :clearable="false"
          placeholder="选择视图"
          class="w-44"
          @update:model-value="applyView"
        />
        <Button variant="outline" size="sm" @click="saveOpen = true">保存为视图</Button>
        <Button v-if="selectedView" variant="outline" size="sm" @click="removeSelectedView">
          删除视图
        </Button>
        <Button variant="ghost" size="sm" @click="clearFilters">清除筛选</Button>
        <Button variant="outline" size="sm" :disabled="rebuilding" @click="rebuild">
          {{ rebuilding ? '重建中…' : '重建索引' }}
        </Button>
        <span class="text-muted-foreground text-xs">{{ items.length }} 条</span>
      </div>

      <p v-if="notice" class="text-muted-foreground text-xs" role="status">{{ notice }}</p>
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
            <EmptyTitle>{{ filtering ? '没有符合条件的条目' : '还没有条目' }}</EmptyTitle>
            <EmptyDescription>
              {{ filtering ? '换个搜索词，或清除筛选条件。' : '点击「新建条目」写下第一条想法。' }}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      </CardContent>
    </Card>

    <ul v-else class="space-y-2">
      <li v-for="item in items" :key="item.id">
        <Card>
          <CardContent class="flex items-center justify-between gap-3">
            <div class="flex min-w-0 flex-col gap-1">
              <div class="flex items-center gap-2">
                <span class="font-medium">{{ item.title }}</span>
                <span
                  v-if="progress[item.id]"
                  class="text-muted-foreground inline-flex items-center rounded-md border px-1.5 py-0.5 text-xs font-medium tabular-nums"
                >
                  {{ progress[item.id].done }}/{{ progress[item.id].total }}
                </span>
              </div>
              <span class="text-muted-foreground text-xs">
                {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} · {{ SPACE_LABELS[item.space] }} ·
                {{ STATUS_LABELS[item.status] }} · {{ item.created.slice(0, 10) }}
              </span>
              <span
                v-if="item.status === 'blocked' || item.status === 'dropped'"
                class="flex flex-wrap items-center gap-1"
              >
                <Badge :variant="item.status === 'blocked' ? 'destructive' : 'outline'">
                  {{ STATUS_LABELS[item.status] }}
                </Badge>
                <span class="text-muted-foreground text-xs">
                  {{ item.status === 'blocked' ? item.blocked_reason : item.dropped_reason }}
                </span>
              </span>
            </div>
            <ItemEditDialog :item="item" @updated="onUpdated" />
          </CardContent>
        </Card>
      </li>
    </ul>

    <Dialog v-model:open="saveOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>保存为视图</DialogTitle>
          <DialogDescription>
            当前筛选条件存进 vault 的 views/ 目录，重启后仍然可用；同名覆盖。
          </DialogDescription>
        </DialogHeader>

        <FormField label="视图名" required>
          <template #default="{ id }">
            <TextInput :id="id" v-model="viewName" placeholder="如：进行中的工作项" />
          </template>
        </FormField>
        <p v-if="saveError" class="text-destructive text-sm" role="alert">{{ saveError }}</p>

        <DialogFooter>
          <DialogClose as-child>
            <Button type="button" variant="outline">取消</Button>
          </DialogClose>
          <Button type="button" :disabled="!viewName.trim()" @click="saveCurrentView">保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </main>
</template>
