<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogScrollContent,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { FormField, SelectField, TextArea, TextInput } from '@/components/form'
import { DatePicker } from '@/components/date-time'
import {
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  PRIORITIES,
  SPACE_LABELS,
  SPACES,
  updateItem,
  setStatus,
  blockItem,
  unblockItem,
  dropItem,
  ALLOWED_STATUSES,
  STATUS_LABELS,
  type Item,
  type ItemEdit,
  type ItemPriority,
  type ItemType,
  type Space,
  type ItemStatus,
} from '@/apis/items'

const props = defineProps<{ item: Item }>()

const emit = defineEmits<{
  updated: [item: Item]
}>()

/** 停止输入 800ms 后落盘；关闭对话框时补一次未落盘的修改 */
const SAVE_DELAY_MS = 800

const open = ref(false)
const saving = ref(false)
const errorMessage = ref('')

const title = ref('')
const type = ref<ItemType>('idea')
const space = ref<Space>('personal')
const project = ref('')
const priority = ref<ItemPriority | ''>('')
const tagsText = ref('')
const due = ref('')
const body = ref('')
const status = ref<ItemStatus>('inbox')

/** 旁路动作对话框：阻塞 / 放弃都要收集必填原因 */
const actionOpen = ref(false)
const action = ref<'block' | 'drop'>('block')
const reasonText = ref('')
const waitingOnText = ref('')
const actionError = ref('')
const actionSaving = ref(false)

/** 最近一次成功落盘的载荷，用来判断有没有未保存的改动 */
const savedPayload = ref('')

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))
const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))
// 只列当前类型允许的状态；blocked / dropped 只展示、不可选，走旁路动作
const statusOptions = computed(() =>
  ALLOWED_STATUSES[props.item.type].map((value) => ({
    value,
    label: STATUS_LABELS[value],
    disabled: value === 'blocked' || value === 'dropped',
  })),
)
const actionValid = computed(
  () =>
    reasonText.value.trim() !== '' &&
    (action.value === 'drop' || waitingOnText.value.trim() !== ''),
)
const titleError = computed(() => (title.value.trim() ? undefined : '标题不能为空'))

function payload(): ItemEdit {
  return {
    title: title.value,
    type: type.value,
    space: space.value,
    project: project.value.trim() || null,
    priority: priority.value || null,
    // 标签用中英文逗号分隔，空白项不落盘
    tags: tagsText.value
      .split(/[,，]/)
      .map((tag) => tag.trim())
      .filter(Boolean),
    due: due.value || null,
    body: body.value,
  }
}

function hydrate(item: Item) {
  title.value = item.title
  type.value = item.type
  space.value = item.space
  project.value = item.project ?? ''
  priority.value = item.priority ?? ''
  tagsText.value = item.tags.join('，')
  due.value = item.due ?? ''
  body.value = item.body
  savedPayload.value = JSON.stringify(payload())
  status.value = item.status
  actionOpen.value = false
  actionError.value = ''
  errorMessage.value = ''
}

async function save() {
  if (!title.value.trim() || JSON.stringify(payload()) === savedPayload.value) return

  saving.value = true
  errorMessage.value = ''
  try {
    const edit = payload()
    const saved = await updateItem(props.item.id, edit)
    savedPayload.value = JSON.stringify(edit)
    // 改类型可能让核心把状态重置为默认值，以落盘结果为准
    status.value = saved.status
    emit('updated', saved)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}

const debouncedSave = useDebounceFn(save, SAVE_DELAY_MS)

watch([title, type, space, project, priority, tagsText, due, body], () => {
  void debouncedSave()
})

watch(open, (value) => {
  if (value) hydrate(props.item)
  else void save()
})

function selectType(value: string) {
  type.value = value as ItemType
}

function selectSpace(value: string) {
  space.value = value as Space
}

function selectPriority(value: string) {
  priority.value = value as ItemPriority | ''
}

async function selectStatus(value: string) {
  const next = value as ItemStatus
  if (!next || next === status.value) return
  errorMessage.value = ''
  try {
    const saved = await setStatus(props.item.id, next)
    status.value = saved.status
    emit('updated', saved)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

async function unblock() {
  errorMessage.value = ''
  try {
    const saved = await unblockItem(props.item.id)
    status.value = saved.status
    emit('updated', saved)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

function openAction(kind: 'block' | 'drop') {
  action.value = kind
  reasonText.value = ''
  waitingOnText.value = ''
  actionError.value = ''
  actionOpen.value = true
}

async function confirmAction() {
  if (!actionValid.value) return
  actionSaving.value = true
  actionError.value = ''
  try {
    const saved =
      action.value === 'block'
        ? await blockItem(props.item.id, reasonText.value, waitingOnText.value)
        : await dropItem(props.item.id, reasonText.value)
    status.value = saved.status
    actionOpen.value = false
    emit('updated', saved)
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error)
  } finally {
    actionSaving.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <Button variant="outline" size="sm">编辑</Button>
    </DialogTrigger>
    <DialogScrollContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>编辑条目</DialogTitle>
        <DialogDescription>
          {{ item.id }} · 修改后自动保存到 vault 的 Markdown 文件
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        <FormField label="标题" required :error="titleError">
          <template #default="{ id, invalid, describedBy, required }">
            <TextInput
              :id="id"
              v-model="title"
              :invalid="invalid"
              :described-by="describedBy"
              :required="required"
              placeholder="一句话说清要做什么"
            />
          </template>
        </FormField>

        <div class="grid gap-4 sm:grid-cols-2">
          <FormField label="类型">
            <template #default="{ id, invalid, describedBy }">
              <SelectField
                :id="id"
                :model-value="type"
                :options="typeOptions"
                :invalid="invalid"
                :described-by="describedBy"
                :clearable="false"
                @update:model-value="selectType"
              />
            </template>
          </FormField>

          <FormField label="状态" hint="可自由跳转；阻塞 / 放弃用下面的旁路动作">
            <template #default="{ id, invalid, describedBy }">
              <SelectField
                :id="id"
                :model-value="status"
                :options="statusOptions"
                :invalid="invalid"
                :described-by="describedBy"
                :clearable="false"
                @update:model-value="selectStatus"
              />
            </template>
          </FormField>

          <FormField label="空间">
            <template #default="{ id, invalid, describedBy }">
              <SelectField
                :id="id"
                :model-value="space"
                :options="spaceOptions"
                :invalid="invalid"
                :described-by="describedBy"
                :clearable="false"
                @update:model-value="selectSpace"
              />
            </template>
          </FormField>

          <FormField label="项目" hint="留空表示不属于任何项目">
            <template #default="{ id, describedBy }">
              <TextInput
                :id="id"
                v-model="project"
                :described-by="describedBy"
                clearable
                placeholder="如：内部报表平台"
              />
            </template>
          </FormField>

          <FormField label="优先级" hint="P0 最高；留空表示未排优先级">
            <template #default="{ id, describedBy }">
              <SelectField
                :id="id"
                :model-value="priority"
                :options="priorityOptions"
                :described-by="describedBy"
                placeholder="未设置"
                @update:model-value="selectPriority"
              />
            </template>
          </FormField>

          <FormField label="标签" hint="多个标签用逗号分隔">
            <template #default="{ id, describedBy }">
              <TextInput
                :id="id"
                v-model="tagsText"
                :described-by="describedBy"
                placeholder="如：报表，后端"
              />
            </template>
          </FormField>

          <FormField label="目标日期" hint="留空表示未排期">
            <template #default="{ id, describedBy }">
              <DatePicker :id="id" v-model="due" :described-by="describedBy" clearable />
            </template>
          </FormField>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <Button
            v-if="status === 'blocked'"
            type="button"
            variant="outline"
            size="sm"
            @click="unblock"
          >
            解除阻塞
          </Button>
          <Button
            v-else-if="status !== 'dropped'"
            type="button"
            variant="outline"
            size="sm"
            @click="openAction('block')"
          >
            阻塞
          </Button>
          <Button
            v-if="status !== 'dropped'"
            type="button"
            variant="outline"
            size="sm"
            @click="openAction('drop')"
          >
            放弃
          </Button>
        </div>

        <p v-if="status === 'blocked'" class="text-muted-foreground text-xs">
          阻塞原因：{{ item.blocked_reason }} · 等待：{{ item.blocked_waiting_on }}
        </p>
        <p v-else-if="status === 'dropped'" class="text-muted-foreground text-xs">
          放弃原因：{{ item.dropped_reason }}
        </p>

        <FormField label="正文" hint="Markdown">
          <template #default="{ id, describedBy }">
            <TextArea
              :id="id"
              v-model="body"
              :rows="12"
              :described-by="describedBy"
              placeholder="背景、验收标准……"
            />
          </template>
        </FormField>

        <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>
      </div>

      <DialogFooter class="sm:items-center sm:justify-between">
        <p class="text-muted-foreground text-xs">{{ saving ? '保存中…' : '修改后自动保存' }}</p>
        <DialogClose as-child>
          <Button type="button" variant="outline">关闭</Button>
        </DialogClose>
      </DialogFooter>
    </DialogScrollContent>
    <Dialog v-model:open="actionOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{{ action === 'block' ? '阻塞条目' : '放弃条目' }}</DialogTitle>
          <DialogDescription>
            {{
              action === 'block'
                ? '阻塞必须填写原因与等待对象；解除阻塞后回到原状态。'
                : '放弃必须填写原因。'
            }}
          </DialogDescription>
        </DialogHeader>

        <div class="space-y-4">
          <FormField label="原因" required>
            <template #default="{ id, describedBy }">
              <TextInput
                :id="id"
                v-model="reasonText"
                :described-by="describedBy"
                :placeholder="action === 'block' ? '如：等运维开通账号' : '如：需求已过时'"
              />
            </template>
          </FormField>

          <FormField v-if="action === 'block'" label="等待对象" required hint="谁或什么在挡着">
            <template #default="{ id, describedBy }">
              <TextInput
                :id="id"
                v-model="waitingOnText"
                :described-by="describedBy"
                placeholder="如：运维组"
              />
            </template>
          </FormField>

          <p v-if="actionError" class="text-destructive text-sm" role="alert">{{ actionError }}</p>
        </div>

        <DialogFooter>
          <DialogClose as-child>
            <Button type="button" variant="outline">取消</Button>
          </DialogClose>
          <Button type="button" :disabled="actionSaving || !actionValid" @click="confirmAction">
            {{ action === 'block' ? '阻塞' : '放弃' }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </Dialog>
</template>
