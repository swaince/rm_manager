<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { CheckboxIndicator, CheckboxRoot } from 'reka-ui'
import { CheckIcon, XIcon } from '@lucide/vue'
import { Badge } from '@/components/ui/badge'
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
import { estimateText } from '@/lib/estimate'
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
  setStatus,
  blockItem,
  unblockItem,
  dropItem,
  ALLOWED_STATUSES,
  ESTIMATES,
  waiveDor,
  type Item,
  type ItemEdit,
  type ItemEstimate,
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

/** 细化（FR-4.1/4.4/4.5）：估算三档、验收标准 checklist、澄清记录 */
const estimate = ref<ItemEstimate | ''>('')
const acceptance = ref<AcceptanceDraft[]>([])
const clarifications = ref<ClarificationDraft[]>([])
const notice = ref('')

interface AcceptanceDraft {
  text: string
  done: boolean
}

/** 澄清的编辑态用空串而非 null：TextInput 只吃字符串，落盘时再转回 null */
interface ClarificationDraft {
  question: string
  answer: string
  source: string
}

/** 豁免就绪检查（FR-4.3）：理由必填 */
const waiveOpen = ref(false)
const waiveReason = ref('')
const waiveError = ref('')
const waiveSaving = ref(false)

/** 旁路动作对话框：阻塞 / 放弃都要收集必填原因 */
const actionOpen = ref(false)
const action = ref<'block' | 'drop'>('block')
const reasonText = ref('')
const waitingOnText = ref('')
const actionError = ref('')
const actionSaving = ref(false)

/** 拆解：父条目 id（'' = 顶层）；子条目清单在打开时加载 */
const parentId = ref('')
const children = ref<Item[]>([])
const parentOptions = ref<{ value: string; label: string }[]>([])

/** 最近一次成功落盘的载荷，用来判断有没有未保存的改动 */
const savedPayload = ref('')

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))
const priorityOptions = PRIORITIES.map((value) => ({ value, label: value }))
const estimateOptions = computed(() =>
  ESTIMATES.map((value) => ({ value, label: estimateText(value) })),
)

/** 就绪检查（FR-4.3）只对需求设门 */
const isRequirement = computed(() => type.value === 'requirement')

/** 未满足项：这里只做提示，进 ready 的硬门与明细消息在 Rust 侧 */
const dorGaps = computed(() => {
  const gaps: string[] = []
  const undone = acceptance.value.filter((criterion) => !criterion.done).length
  if (!acceptance.value.length) gaps.push('验收标准未填写')
  else if (undone) gaps.push(`验收标准还有 ${undone} 条未勾选`)
  if (!estimate.value) gaps.push('未估算')
  const unanswered = clarifications.value.filter((one) => !one.answer.trim()).length
  if (unanswered) gaps.push(`澄清问题还有 ${unanswered} 条待答复`)
  return gaps
})
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
    estimate: estimate.value || null,
    acceptance: acceptance.value.map((criterion) => ({ ...criterion })),
    clarifications: clarifications.value.map((clarification) => ({
      question: clarification.question,
      answer: clarification.answer.trim() || null,
      source: clarification.source.trim() || null,
    })),
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
  estimate.value = item.estimate ?? ''
  acceptance.value = item.acceptance.map((criterion) => ({ ...criterion }))
  clarifications.value = item.clarifications.map((clarification) => ({
    question: clarification.question,
    answer: clarification.answer ?? '',
    source: clarification.source ?? '',
  }))
  notice.value = ''
  waiveOpen.value = false
  waiveError.value = ''
  body.value = item.body
  savedPayload.value = JSON.stringify(payload())
  status.value = item.status
  actionOpen.value = false
  actionError.value = ''
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

watch(
  [
    title,
    type,
    space,
    project,
    priority,
    tagsText,
    due,
    body,
    estimate,
    acceptance,
    clarifications,
  ],
  () => {
    void debouncedSave()
  },
  { deep: true },
)

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

function selectEstimate(value: string) {
  estimate.value = value as ItemEstimate | ''
}

function addAcceptance() {
  acceptance.value.push({ text: '', done: false })
}

function addClarification() {
  clarifications.value.push({ question: '', answer: '', source: '' })
}

/** 细化模板（FR-4.1）：背景 / 目标 / 范围 / 验收标准 / 边界与异常 五段骨架 */
const TEMPLATE_HEADINGS = ['## 背景', '## 目标', '## 范围', '## 验收标准', '## 边界与异常']
const REFINEMENT_TEMPLATE = `## 背景

（要解决什么问题？为什么现在做？）

## 目标

（做到什么算成功？）

## 范围

- 做什么：
- 不做什么：

## 验收标准

- [ ]

## 边界与异常

（边界条件、异常路径、明确的例外）`

function insertTemplate() {
  if (TEMPLATE_HEADINGS.some((heading) => body.value.includes(heading))) {
    notice.value = '正文里已有细化模板段落，未重复插入'
    return
  }
  const existing = body.value.trimEnd()
  body.value = existing ? `${existing}\n\n${REFINEMENT_TEMPLATE}\n` : `${REFINEMENT_TEMPLATE}\n`
  notice.value = '已插入细化模板'
}

function openWaive() {
  waiveReason.value = ''
  waiveError.value = ''
  waiveOpen.value = true
}

async function confirmWaive() {
  if (!waiveReason.value.trim()) return
  waiveSaving.value = true
  waiveError.value = ''
  try {
    const saved = await waiveDor(props.item.id, waiveReason.value)
    waiveOpen.value = false
    emit('updated', saved)
  } catch (error) {
    waiveError.value = error instanceof Error ? error.message : String(error)
  } finally {
    waiveSaving.value = false
  }
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

          <FormField label="估算" hint="三档预算；单位在列表页设置里切换人日 / 点数">
            <template #default="{ id, describedBy }">
              <SelectField
                :id="id"
                :model-value="estimate"
                :options="estimateOptions"
                :described-by="describedBy"
                placeholder="未估算"
                @update:model-value="selectEstimate"
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

        <section v-if="isRequirement" class="space-y-2 rounded-lg border p-3" aria-label="就绪检查">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-medium">就绪检查（DoR）</h3>
            <Button
              v-if="!item.dor_waived"
              type="button"
              variant="outline"
              size="sm"
              @click="openWaive"
            >
              豁免
            </Button>
          </div>
          <p v-if="item.dor_waived" class="text-muted-foreground text-xs">
            已豁免：{{ item.dor_waived }}（进 ready 免检，理由已留痕）
          </p>
          <p v-else-if="!dorGaps.length" class="text-muted-foreground text-xs">
            全部满足，可进 ready
          </p>
          <ul v-else class="text-muted-foreground list-disc space-y-1 pl-5 text-xs">
            <li v-for="gap in dorGaps" :key="gap">{{ gap }}</li>
          </ul>
        </section>

        <FormField label="正文" hint="Markdown">
          <template #default="{ id, describedBy }">
            <div class="space-y-2">
              <div class="flex flex-wrap items-center gap-2">
                <Button type="button" variant="outline" size="sm" @click="insertTemplate">
                  插入细化模板
                </Button>
                <span v-if="notice" class="text-muted-foreground text-xs" role="status">
                  {{ notice }}
                </span>
              </div>
              <TextArea
                :id="id"
                v-model="body"
                :rows="12"
                :described-by="describedBy"
                placeholder="背景、验收标准……"
              />
            </div>
          </template>
        </FormField>

        <FormField label="验收标准" hint="checklist：全部勾选才满足就绪检查">
          <template #default="{ describedBy }">
            <div class="space-y-2">
              <div
                v-for="(criterion, index) in acceptance"
                :key="index"
                class="flex items-center gap-2"
              >
                <CheckboxRoot
                  v-model="criterion.done"
                  :aria-label="`验收标准 ${index + 1}`"
                  :aria-describedby="describedBy"
                  class="border-input data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground size-4 shrink-0 rounded-[4px] border"
                >
                  <CheckboxIndicator class="flex items-center justify-center">
                    <CheckIcon class="size-3.5" />
                  </CheckboxIndicator>
                </CheckboxRoot>
                <TextInput
                  v-model="criterion.text"
                  :described-by="describedBy"
                  placeholder="如：导出 1000 行不超过 5 秒"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  aria-label="删除验收标准"
                  @click="acceptance.splice(index, 1)"
                >
                  <XIcon class="size-4" />
                </Button>
              </div>
              <Button type="button" variant="outline" size="sm" @click="addAcceptance">
                添加验收标准
              </Button>
            </div>
          </template>
        </FormField>

        <FormField label="澄清记录" hint="答复留空即待答复；未清零则就绪检查不通过">
          <template #default="{ describedBy }">
            <div class="space-y-3">
              <div
                v-for="(clarification, index) in clarifications"
                :key="index"
                class="space-y-2 rounded-lg border p-3"
              >
                <div class="flex items-center gap-2">
                  <Badge :variant="clarification.answer.trim() ? 'secondary' : 'outline'">
                    {{ clarification.answer.trim() ? '已答复' : '待答复' }}
                  </Badge>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-xs"
                    class="ml-auto"
                    aria-label="删除澄清记录"
                    @click="clarifications.splice(index, 1)"
                  >
                    <XIcon class="size-4" />
                  </Button>
                </div>
                <TextInput
                  v-model="clarification.question"
                  :described-by="describedBy"
                  placeholder="问题，如：历史数据要不要一起迁移"
                />
                <TextInput
                  v-model="clarification.answer"
                  :described-by="describedBy"
                  placeholder="答复（留空 = 待答复）"
                />
                <TextInput
                  v-model="clarification.source"
                  :described-by="describedBy"
                  placeholder="来源人，如：产品"
                />
              </div>
              <Button type="button" variant="outline" size="sm" @click="addClarification">
                添加澄清问题
              </Button>
            </div>
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
    <Dialog v-model:open="waiveOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>豁免就绪检查</DialogTitle>
          <DialogDescription>
            未满足的就绪检查项会被跳过；理由写进条目 frontmatter 留痕，之后进 ready 不再拦截。
          </DialogDescription>
        </DialogHeader>

        <div class="space-y-4">
          <FormField label="豁免理由" required>
            <template #default="{ id, describedBy }">
              <TextInput
                :id="id"
                v-model="waiveReason"
                :described-by="describedBy"
                placeholder="如：本季度先做，澄清在排期后补"
              />
            </template>
          </FormField>

          <p v-if="waiveError" class="text-destructive text-sm" role="alert">{{ waiveError }}</p>
        </div>

        <DialogFooter>
          <DialogClose as-child>
            <Button type="button" variant="outline">取消</Button>
          </DialogClose>
          <Button
            type="button"
            :disabled="waiveSaving || !waiveReason.trim()"
            @click="confirmWaive"
          >
            豁免
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </Dialog>
</template>
