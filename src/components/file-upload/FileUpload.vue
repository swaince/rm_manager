<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue'
import { FileIcon, UploadIcon } from '@lucide/vue'
import {
  Attachment,
  AttachmentAction,
  AttachmentActions,
  AttachmentContent,
  AttachmentDescription,
  AttachmentMedia,
  AttachmentTitle,
} from '@/components/ui/attachment'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Progress } from '@/components/ui/progress'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Spinner } from '@/components/ui/spinner'
import { cn } from '@/lib/utils'

export type FileUploadStatus = 'uploading' | 'success' | 'error' | 'cancelled'

export interface FileUploadHandlerOptions {
  signal: AbortSignal
  onProgress: (progress: number) => void
}

export type FileUploadHandler = (file: File, options: FileUploadHandlerOptions) => Promise<unknown>

export interface FileUploadItem {
  id: string
  file: File
  status: FileUploadStatus
  progress: number | null
  response?: unknown
  error?: string
}

export interface FileUploadRejection {
  file: File
  reason: string
}

const props = withDefaults(
  defineProps<{
    upload: FileUploadHandler
    id?: string
    describedBy?: string
    description?: string
    accept?: string
    multiple?: boolean
    disabled?: boolean
    maxVisibleItems?: number
    class?: HTMLAttributes['class']
  }>(),
  {
    multiple: true,
    disabled: false,
    maxVisibleItems: 10,
  },
)

const emit = defineEmits<{
  (event: 'change', items: FileUploadItem[]): void
  (event: 'reject', rejection: FileUploadRejection): void
}>()

const generatedId = useId()
const inputId = computed(() => props.id?.trim() || `file-upload-${generatedId}`)
const descriptionId = computed(() => `${inputId.value}-description`)
const inputDescribedBy = computed(
  () =>
    [props.describedBy, props.description ? descriptionId.value : undefined]
      .filter(Boolean)
      .join(' ') || undefined,
)
const items = ref<FileUploadItem[]>([])
const rejectionMessages = ref<string[]>([])
const inputKey = ref(0)
const inputRef = ref<InstanceType<typeof Input> | null>(null)
const dropzoneButtonRef = ref<InstanceType<typeof Button> | null>(null)
const isDragging = ref(false)
const controllers = new Map<string, AbortController>()
let sequence = 0
let disposed = false
let dragDepth = 0
let restorePickerFocus = false

const normalizedMaxVisibleItems = computed(() =>
  Number.isSafeInteger(props.maxVisibleItems) && props.maxVisibleItems > 0
    ? props.maxVisibleItems
    : 10,
)
const isListScrollable = computed(() => items.value.length > normalizedMaxVisibleItems.value)
const listItemRefs = ref<HTMLLIElement[]>([])
const listViewportHeight = ref(0)
let listResizeObserver: ResizeObserver | undefined
const observedListItems = new Set<HTMLLIElement>()

function visibleListItems() {
  return listItemRefs.value
    .slice(0, normalizedMaxVisibleItems.value)
    .filter((element): element is HTMLLIElement => element !== null)
}

function measureListViewport() {
  if (!isListScrollable.value) {
    listViewportHeight.value = 0
    return
  }

  const height = visibleListItems().reduce(
    (total, element) => total + element.getBoundingClientRect().height,
    0,
  )
  listViewportHeight.value = height > 0 ? Math.ceil(height) : 0
}

function syncListItemMeasurements() {
  const nextItems = new Set(visibleListItems())
  for (const element of observedListItems) {
    if (nextItems.has(element)) continue
    listResizeObserver?.unobserve(element)
    observedListItems.delete(element)
  }
  for (const element of nextItems) {
    if (observedListItems.has(element)) continue
    listResizeObserver?.observe(element)
    observedListItems.add(element)
  }
  measureListViewport()
}

watch(
  () => [
    normalizedMaxVisibleItems.value,
    ...items.value.map(({ id, status, error }) => `${id}:${status}:${error ?? ''}`),
  ],
  syncListItemMeasurements,
  { flush: 'post' },
)

onMounted(() => {
  if (typeof ResizeObserver !== 'undefined')
    listResizeObserver = new ResizeObserver(measureListViewport)
  syncListItemMeasurements()
})

onBeforeUnmount(() => {
  listResizeObserver?.disconnect()
  observedListItems.clear()
})

function publishChange() {
  emit(
    'change',
    items.value.map((item) => ({ ...item })),
  )
}

async function uploadItem(item: FileUploadItem) {
  const controller = new AbortController()
  controllers.set(item.id, controller)
  item.status = 'uploading'
  item.progress = null
  delete item.error
  delete item.response
  publishChange()

  try {
    const response = await props.upload(item.file, {
      signal: controller.signal,
      onProgress: (progress) => {
        if (!Number.isFinite(progress) || controllers.get(item.id) !== controller || disposed)
          return
        item.progress = Math.min(100, Math.max(0, progress))
        publishChange()
      },
    })

    if (controllers.get(item.id) !== controller || controller.signal.aborted || disposed) return
    item.status = 'success'
    item.response = response
  } catch (error) {
    if (controllers.get(item.id) !== controller || controller.signal.aborted || disposed) return
    item.status = 'error'
    item.error = error instanceof Error ? error.message : String(error)
  } finally {
    const isCurrentRequest = controllers.get(item.id) === controller
    if (isCurrentRequest) controllers.delete(item.id)
    if (isCurrentRequest && !disposed) publishChange()
  }
}

function isAccepted(file: File) {
  const filters =
    props.accept
      ?.split(',')
      .map((filter) => filter.trim().toLowerCase())
      .filter(Boolean) ?? []
  if (!filters.length) return true

  const name = file.name.toLowerCase()
  const type = file.type.toLowerCase()
  return filters.some((filter) => {
    if (filter.startsWith('.')) return name.endsWith(filter)
    if (filter.endsWith('/*')) return type.startsWith(filter.slice(0, -1))
    return type === filter
  })
}

function statusText(item: FileUploadItem) {
  if (item.status === 'uploading') {
    return item.progress === null ? '上传中' : `上传中 · ${Math.round(item.progress)}%`
  }
  if (item.status === 'success') return '已上传'
  if (item.status === 'error') return '上传失败'
  return '已取消'
}

function attachmentState(status: FileUploadStatus) {
  if (status === 'success') return 'done'
  if (status === 'error') return 'error'
  if (status === 'uploading') return 'uploading'
  return 'idle'
}

function formatFileSize(size: number) {
  if (size < 1024) return `${size} B`

  const units = ['KB', 'MB', 'GB', 'TB']
  let value = size
  let unitIndex = -1
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }

  return `${new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 1 }).format(value)} ${units[unitIndex]}`
}

function cancelItem(item: FileUploadItem) {
  if (props.disabled) return

  const controller = controllers.get(item.id)
  if (!controller) return

  controllers.delete(item.id)
  item.status = 'cancelled'
  controller.abort()
  publishChange()
}

function retryItem(item: FileUploadItem) {
  if (props.disabled || (item.status !== 'error' && item.status !== 'cancelled')) return
  void uploadItem(item)
}

function removeItem(item: FileUploadItem) {
  if (props.disabled) return

  const controller = controllers.get(item.id)
  if (controller) {
    controllers.delete(item.id)
    controller.abort()
  }

  items.value = items.value.filter((current) => current.id !== item.id)
  publishChange()
}

function addFiles(files: File[]) {
  rejectionMessages.value = []
  if (props.disabled) return

  let acceptedCount = 0
  for (const file of files) {
    if (!isAccepted(file)) {
      const reason = '不支持的文件类型'
      rejectionMessages.value.push(`${file.name}：${reason}`)
      emit('reject', { file, reason })
      continue
    }
    if (!props.multiple && acceptedCount > 0) {
      const reason = '一次只能选择一个文件'
      rejectionMessages.value.push(`${file.name}：${reason}`)
      emit('reject', { file, reason })
      continue
    }

    acceptedCount += 1
    const item: FileUploadItem = {
      id: `${inputId.value}-${++sequence}`,
      file,
      status: 'uploading',
      progress: null,
    }
    items.value.push(item)
    const reactiveItem = items.value[items.value.length - 1]
    if (reactiveItem) void uploadItem(reactiveItem)
  }
}

function focusDropzone() {
  void nextTick(() => {
    const button = dropzoneButtonRef.value?.$el
    if (!disposed && button instanceof HTMLButtonElement) button.focus({ preventScroll: true })
  })
}

function openFileDialog() {
  if (props.disabled) return
  const input = inputRef.value?.$el
  if (!(input instanceof HTMLInputElement)) return

  restorePickerFocus = true
  input.click()
}

function onFileChange(event: Event) {
  const input = event.target as HTMLInputElement
  const shouldRestoreFocus = restorePickerFocus || document.activeElement === input
  const files = Array.from(input.files ?? [])
  input.value = ''
  inputKey.value += 1
  restorePickerFocus = false

  if (shouldRestoreFocus) focusDropzone()
  addFiles(files)
}

function onFileCancel() {
  if (!restorePickerFocus) return
  restorePickerFocus = false
  focusDropzone()
}

function onDragEnter(event: DragEvent) {
  event.preventDefault()
  if (props.disabled) return
  dragDepth += 1
  isDragging.value = true
}

function onDragOver(event: DragEvent) {
  event.preventDefault()
  if (event.dataTransfer) event.dataTransfer.dropEffect = props.disabled ? 'none' : 'copy'
}

function onDragLeave(event: DragEvent) {
  event.preventDefault()
  dragDepth = Math.max(0, dragDepth - 1)
  isDragging.value = dragDepth > 0 && !props.disabled
}

function onDrop(event: DragEvent) {
  event.preventDefault()
  dragDepth = 0
  isDragging.value = false
  addFiles(Array.from(event.dataTransfer?.files ?? []))
}

onBeforeUnmount(() => {
  disposed = true
  for (const controller of controllers.values()) controller.abort()
  controllers.clear()
})
</script>

<template>
  <div :class="cn('flex w-full min-w-0 flex-col gap-3', props.class)">
    <Input
      :key="inputKey"
      ref="inputRef"
      :id="inputId"
      type="file"
      :accept="accept"
      :multiple="multiple"
      :disabled="disabled"
      :model-value="''"
      :aria-describedby="inputDescribedBy"
      tabindex="-1"
      aria-hidden="true"
      class="hidden"
      @change="onFileChange"
      @cancel="onFileCancel"
    />

    <Button
      ref="dropzoneButtonRef"
      data-slot="file-upload-dropzone"
      :data-dragging="isDragging && !disabled"
      :data-disabled="disabled"
      type="button"
      variant="ghost"
      :disabled="disabled"
      aria-label="点击或拖拽文件到这里"
      :aria-controls="inputId"
      :aria-describedby="inputDescribedBy"
      :class="
        cn(
          'flex h-auto min-h-56 w-full min-w-0 flex-col items-center justify-center gap-3 whitespace-normal rounded-lg border border-dashed border-input bg-muted/20 px-6 py-8 text-center text-base font-normal text-foreground shadow-none transition-colors',
          !disabled && 'hover:border-primary/60 hover:bg-accent/30 hover:text-foreground',
          isDragging && !disabled && 'border-primary bg-accent/50',
          disabled && 'cursor-not-allowed opacity-60',
        )
      "
      @click="openFileDialog"
      @dragenter="onDragEnter"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
      @drop="onDrop"
    >
      <span
        class="flex size-14 items-center justify-center rounded-full bg-muted text-muted-foreground"
      >
        <UploadIcon class="size-5" aria-hidden="true" />
      </span>
      <span class="min-w-0 space-y-1">
        <span class="block font-medium">点击或拖拽文件到这里</span>
        <span v-if="description" :id="descriptionId" class="block text-sm text-muted-foreground">
          {{ description }}
        </span>
      </span>
    </Button>

    <div v-if="rejectionMessages.length" class="space-y-1" aria-live="polite">
      <p
        v-for="message in rejectionMessages"
        :key="message"
        role="alert"
        class="text-sm break-all text-destructive"
      >
        {{ message }}
      </p>
    </div>

    <div
      v-if="items.length"
      data-slot="file-upload-list-scroll"
      :style="
        isListScrollable && listViewportHeight > 0
          ? { height: listViewportHeight + 'px' }
          : undefined
      "
      class="w-full min-w-0"
    >
      <ScrollArea :class="isListScrollable ? 'h-full w-full min-w-0' : 'w-full min-w-0'">
        <ul
          role="list"
          aria-label="上传文件列表"
          class="flex w-full min-w-0 flex-col divide-y divide-border"
        >
          <li
            v-for="item in items"
            ref="listItemRefs"
            :key="item.id"
            role="listitem"
            class="min-w-0 py-3 first:pt-0 last:pb-0"
          >
            <Attachment
              :state="attachmentState(item.status)"
              class="w-full shrink flex-nowrap rounded-none border-0 bg-transparent p-0 shadow-none hover:bg-transparent"
            >
              <AttachmentMedia class="rounded-md">
                <FileIcon aria-hidden="true" />
              </AttachmentMedia>

              <AttachmentContent class="px-0 py-0">
                <div class="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
                  <div class="min-w-0 flex-1">
                    <AttachmentTitle :title="item.file.name">
                      {{ item.file.name }}
                    </AttachmentTitle>
                    <AttachmentDescription>
                      {{ formatFileSize(item.file.size) }}
                    </AttachmentDescription>
                    <p
                      v-if="item.status === 'error'"
                      class="mt-1 break-all text-xs text-destructive"
                      aria-live="polite"
                    >
                      {{ item.error || '未知错误' }}
                    </p>
                    <Progress
                      v-if="item.status === 'uploading' && item.progress !== null"
                      :model-value="item.progress"
                      :aria-label="`上传进度：${item.file.name}`"
                      class="mt-2"
                    />
                  </div>

                  <span
                    class="flex shrink-0 items-center gap-1.5 whitespace-nowrap text-xs text-muted-foreground sm:ml-auto"
                    aria-live="off"
                  >
                    <Spinner
                      v-if="item.status === 'uploading' && item.progress === null"
                      class="size-3.5"
                      aria-hidden="true"
                    />
                    {{ statusText(item) }}
                  </span>

                  <AttachmentActions class="shrink-0 self-start sm:self-auto">
                    <AttachmentAction
                      v-if="item.status === 'uploading'"
                      size="sm"
                      :aria-label="`取消 ${item.file.name}`"
                      :title="`取消 ${item.file.name}`"
                      :disabled="disabled"
                      type="button"
                      @click="cancelItem(item)"
                    >
                      取消
                    </AttachmentAction>
                    <AttachmentAction
                      v-if="item.status === 'error' || item.status === 'cancelled'"
                      size="sm"
                      :aria-label="`重试 ${item.file.name}`"
                      :title="`重试 ${item.file.name}`"
                      :disabled="disabled"
                      type="button"
                      @click="retryItem(item)"
                    >
                      重试
                    </AttachmentAction>
                    <AttachmentAction
                      v-if="item.status !== 'uploading'"
                      size="sm"
                      :aria-label="`移除 ${item.file.name}`"
                      :title="`移除 ${item.file.name}`"
                      :disabled="disabled"
                      type="button"
                      @click="removeItem(item)"
                    >
                      移除
                    </AttachmentAction>
                  </AttachmentActions>
                </div>
              </AttachmentContent>
            </Attachment>
          </li>
        </ul>
      </ScrollArea>
    </div>
  </div>
</template>
