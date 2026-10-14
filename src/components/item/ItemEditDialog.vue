<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
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
  childrenOf,
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  listItems,
  PRIORITIES,
  setParent,
  SPACE_LABELS,
  SPACES,
  STATUS_LABELS,
  updateItem,
  type Item,
  type ItemEdit,
  type ItemPriority,
  type ItemType,
  type Space,
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

/** 拆解：父条目 id（'' = 顶层）；子条目清单在打开时加载 */
const parentId = ref('')
const children = ref<Item[]>([])
const parentOptions = ref<{ value: string; label: string }[]>([])

/** 最近一次成功落盘的载荷，用来判断有没有未保存的改动 */
const savedPayload = ref('')

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))
const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))
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
  parentId.value = item.parent ?? ''
  body.value = item.body
  savedPayload.value = JSON.stringify(payload())
  errorMessage.value = ''
}

/** 打开时加载：可选父条目（顶层条目，排除自己）与当前条目的子条目清单 */
async function loadRelations() {
  try {
    const all = await listItems()
    parentOptions.value = all
      .filter((item) => !item.parent && item.id !== props.item.id)
      .map((item) => ({ value: item.id, label: `${item.id}--${item.title}` }))
    children.value = await childrenOf(props.item.id)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

/** 父条目是动作级修改：选中即落盘，不走正文的防抖 */
async function selectParent(value: string) {
  parentId.value = value
  try {
    const saved = await setParent(props.item.id, value || null)
    emit('updated', saved)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

/** 子条目在自己的对话框里改动后：刷新清单，并把最新条目回传给列表 */
async function onChildUpdated(updated: Item) {
  emit('updated', updated)
  children.value = await childrenOf(props.item.id)
}

async function save() {
  if (!title.value.trim() || JSON.stringify(payload()) === savedPayload.value) return

  saving.value = true
  errorMessage.value = ''
  try {
    const edit = payload()
    const saved = await updateItem(props.item.id, edit)
    savedPayload.value = JSON.stringify(edit)
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
  if (value) {
    hydrate(props.item)
    void loadRelations()
  } else void save()
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
</script>

<template>
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <slot name="trigger">
        <Button variant="outline" size="sm">编辑</Button>
      </slot>
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

          <FormField label="父条目" hint="拆解：留空表示顶层条目">
            <template #default="{ id, describedBy }">
              <SelectField
                :id="id"
                :model-value="parentId"
                :options="parentOptions"
                :described-by="describedBy"
                placeholder="顶层条目"
                @update:model-value="selectParent"
              />
            </template>
          </FormField>
        </div>

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
        <section v-if="children.length" class="space-y-2">
          <h3 class="text-sm font-medium">子条目（{{ children.length }}）</h3>
          <ul class="space-y-1">
            <li v-for="child in children" :key="child.id">
              <ItemEditDialog :item="child" @updated="onChildUpdated">
                <template #trigger>
                  <Button variant="link" size="sm" class="h-auto justify-start px-0">
                    {{ child.id }}--{{ child.title }}（{{ STATUS_LABELS[child.status] }}）
                  </Button>
                </template>
              </ItemEditDialog>
            </li>
          </ul>
        </section>
      </div>

      <DialogFooter class="sm:items-center sm:justify-between">
        <p class="text-muted-foreground text-xs">{{ saving ? '保存中…' : '修改后自动保存' }}</p>
        <DialogClose as-child>
          <Button type="button" variant="outline">关闭</Button>
        </DialogClose>
      </DialogFooter>
    </DialogScrollContent>
  </Dialog>
</template>
