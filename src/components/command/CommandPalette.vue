<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { emit as broadcast } from '@tauri-apps/api/event'
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from '@/components/ui/command'
import {
  ALLOWED_STATUSES,
  PRIORITIES,
  STATUS_LABELS,
  listItems,
  scheduleItem,
  setStatus,
  updateItem,
  type Item,
  type ItemPriority,
  type ItemStatus,
} from '@/apis/items'
import { syncNow } from '@/apis/sync'
import { ITEMS_CHANGED } from '@/lib/capture'
import { addDays, formatDate, mondayOf } from '@/lib/date'

/**
 * 命令面板（FR-0.7）：Ctrl+K 呼出，新建 / 跳转 / 改状态 / 排期 / 同步全覆盖；
 * 配合既有视图把「捕获 → 整理 → 改状态 → 排期」串成全程无鼠标链路。
 */

/** 子页动作：先选条目再选值；edit 例外——选中即打开条目编辑 */
type Action = 'edit' | 'status' | 'priority' | 'schedule'

const props = defineProps<{ open?: boolean }>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  navigate: [tab: 'items' | 'inbox' | 'planning']
  edit: [item: Item]
  create: []
}>()

/** 受控模式（外部传 open 时）：跟随 prop；非受控：内部自管 */
const innerOpen = ref(false)
const open = computed({
  get: () => props.open ?? innerOpen.value,
  set: (value: boolean) => {
    innerOpen.value = value
    emit('update:open', value)
  },
})

const items = ref<Item[]>([])
const errorMessage = ref('')
const action = ref<Action | null>(null)
const picked = ref<Item | null>(null)
const busy = ref(false)

const today = formatDate(new Date())
const thisMonday = mondayOf(today)
const nextMonday = addDays(thisMonday, 7)

async function loadItems() {
  try {
    items.value = await listItems()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

/** 每次打开都重置到根页并刷新条目列表，保证看到的是最新数据 */
watch(open, (value) => {
  if (!value) return
  action.value = null
  picked.value = null
  errorMessage.value = ''
  void loadItems()
})

function close() {
  open.value = false
  action.value = null
  picked.value = null
}

/** Ctrl+K 呼出 / 收起（窗口内监听，只在主窗注册） */
function onKeydown(event: KeyboardEvent) {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return
  if (event.key.toLowerCase() !== 'k') return
  event.preventDefault()
  open.value = !open.value
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))

function enter(next: Action) {
  action.value = next
  picked.value = null
}

const actionHeading = computed(() => {
  switch (action.value) {
    case 'edit':
      return '选择要打开的条目'
    case 'status':
      return '选择条目，下一步改状态'
    case 'priority':
      return '选择条目，下一步改优先级'
    case 'schedule':
      return '选择条目，下一步排期'
    default:
      return ''
  }
})

function pick(item: Item) {
  // 跳转条目不需要第二步：选中即交给主界面打开编辑对话框
  if (action.value === 'edit') {
    emit('edit', item)
    close()
    return
  }
  picked.value = item
}

function go(tab: 'items' | 'inbox' | 'planning') {
  emit('navigate', tab)
  close()
}

function openCreate() {
  emit('create')
  close()
}

/** 数据改动完成后广播：列表 / 整理台 / 规划等监听方各自刷新 */
async function finish() {
  await broadcast(ITEMS_CHANGED)
  close()
}

async function run(fn: () => Promise<Item>) {
  if (busy.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    await fn()
    await finish()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    busy.value = false
  }
}

/** blocked / dropped 必须填原因，是旁路动作，走条目编辑里的专门入口 */
const statusChoices = computed(() => {
  if (!picked.value) return []
  return ALLOWED_STATUSES[picked.value.type]
    .filter((status) => status !== 'blocked' && status !== 'dropped')
    .map((status) => ({ value: status, label: STATUS_LABELS[status] }))
})

const priorityChoices = computed(() => {
  const choices: { value: string; label: string }[] = PRIORITIES.map((value) => ({
    value,
    label: `优先级 ${value}`,
  }))
  if (picked.value?.priority) choices.push({ value: '', label: '清除优先级' })
  return choices
})

const scheduleChoices = [
  { value: thisMonday, label: `排入本周（${thisMonday} 起）` },
  { value: nextMonday, label: `排入下周（${nextMonday} 起）` },
  { value: '', label: '清除排期' },
]

async function applyStatus(value: ItemStatus) {
  const item = picked.value
  if (!item || value === item.status) return
  await run(() => setStatus(item.id, value))
}

/** 改优先级：核心 update_item 收全量编辑载荷，其余字段保持原值（与列表行内改法一致） */
async function applyPriority(value: string) {
  const item = picked.value
  if (!item) return
  const priority = (value || null) as ItemPriority | null
  if (priority === item.priority) return
  await run(() =>
    updateItem(item.id, {
      title: item.title,
      type: item.type,
      space: item.space,
      project: item.project,
      priority,
      tags: item.tags,
      due: item.due,
      estimate: item.estimate,
      acceptance: item.acceptance,
      clarifications: item.clarifications,
      body: item.body,
    }),
  )
}

async function applySchedule(value: string) {
  const item = picked.value
  if (!item) return
  await run(() => scheduleItem(item.id, value || null))
}

async function runSync() {
  if (busy.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    await syncNow()
    close()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <CommandDialog
    v-model:open="open"
    title="命令面板"
    description="Ctrl+K 呼出；输入即筛选，回车执行"
  >
    <CommandInput placeholder="输入命令：新建 / 跳转 / 改状态 / 排期 / 同步" />
    <CommandList>
      <CommandEmpty>没有匹配的命令或条目</CommandEmpty>

      <template v-if="!action">
        <CommandGroup heading="动作">
          <CommandItem value="新建条目" @select="openCreate">
            新建条目
            <CommandShortcut>Ctrl+Shift+Space 速记</CommandShortcut>
          </CommandItem>
          <CommandItem value="立即同步" @select="runSync">立即同步</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="条目操作">
          <CommandItem value="改状态" @select="enter('status')">改状态…</CommandItem>
          <CommandItem value="改优先级" @select="enter('priority')">改优先级…</CommandItem>
          <CommandItem value="排期" @select="enter('schedule')">排期…</CommandItem>
          <CommandItem value="打开条目" @select="enter('edit')">打开条目…</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="跳转视图">
          <CommandItem value="跳到条目列表" @select="go('items')">跳到「条目」</CommandItem>
          <CommandItem value="跳到收件箱" @select="go('inbox')">跳到「收件箱」</CommandItem>
          <CommandItem value="跳到规划" @select="go('planning')">跳到「规划」</CommandItem>
        </CommandGroup>
      </template>

      <template v-else-if="!picked">
        <CommandGroup :heading="actionHeading">
          <CommandItem value="返回命令列表" @select="action = null">← 返回命令列表</CommandItem>
          <CommandItem
            v-for="item in items"
            :key="item.id"
            :value="`${item.title} ${item.id}`"
            @select="pick(item)"
          >
            <span class="truncate">{{ item.title }}</span>
            <CommandShortcut>{{ item.id }}</CommandShortcut>
          </CommandItem>
        </CommandGroup>
      </template>

      <template v-else>
        <CommandGroup :heading="picked.title">
          <CommandItem value="返回条目列表" @select="picked = null">← 返回条目列表</CommandItem>
          <template v-if="action === 'status'">
            <CommandItem
              v-for="choice in statusChoices"
              :key="choice.value"
              :value="choice.label"
              @select="applyStatus(choice.value)"
            >
              {{ choice.label }}
            </CommandItem>
          </template>
          <template v-else-if="action === 'priority'">
            <CommandItem
              v-for="choice in priorityChoices"
              :key="choice.value || 'clear'"
              :value="choice.label"
              @select="applyPriority(choice.value)"
            >
              {{ choice.label }}
            </CommandItem>
          </template>
          <template v-else-if="action === 'schedule'">
            <CommandItem
              v-for="choice in scheduleChoices"
              :key="choice.value || 'clear'"
              :value="choice.label"
              @select="applySchedule(choice.value)"
            >
              {{ choice.label }}
            </CommandItem>
          </template>
        </CommandGroup>
      </template>

      <p v-if="errorMessage" class="text-destructive px-3 py-2 text-sm" role="alert">
        {{ errorMessage }}
      </p>
    </CommandList>
  </CommandDialog>
</template>
