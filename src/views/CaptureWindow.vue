<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { emit } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Button } from '@/components/ui/button'
import { SelectField, TextArea, TextInput } from '@/components/form'
import {
  createItem,
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  SPACE_LABELS,
  SPACES,
  type ItemType,
  type Space,
} from '@/apis/items'
import { focusMainWindow, hideCaptureWindow, ITEMS_CHANGED } from '@/lib/capture'

/** 迷你速记窗：标题可空（核心取正文首行兜底），全字段可空；Esc 关闭、提交后关闭并聚焦主窗 */

const title = ref('')
const type = ref<ItemType>('idea')
const space = ref<Space>('personal')
const body = ref('')
const submitting = ref(false)
const errorMessage = ref('')

const formRoot = ref<HTMLElement | null>(null)

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))

function selectType(value: string) {
  type.value = value as ItemType
}

function selectSpace(value: string) {
  space.value = value as Space
}

async function hide() {
  await hideCaptureWindow()
}

async function submit() {
  if (submitting.value) return
  submitting.value = true
  errorMessage.value = ''
  try {
    await createItem({ title: title.value, type: type.value, space: space.value, body: body.value })
    await emit(ITEMS_CHANGED)
    title.value = ''
    body.value = ''
    await hide()
    await focusMainWindow()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    submitting.value = false
  }
}

function onKeydown(event: KeyboardEvent) {
  // Esc 只收起窗口，草稿留着下次回来
  if (event.key === 'Escape') void hide()
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  // 每次被快捷键唤起都聚焦正文，进来就能直接打字
  void getCurrentWebviewWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) formRoot.value?.querySelector('textarea')?.focus()
  })
})

onUnmounted(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <form class="bg-background flex h-screen flex-col gap-3 p-4" novalidate @submit.prevent="submit">
    <p class="text-muted-foreground text-xs" data-tauri-drag-region>
      速记 · 存为收件箱条目 · Esc 收起
    </p>

    <div ref="formRoot" class="flex min-h-0 flex-1 flex-col gap-3">
      <TextInput v-model="title" placeholder="标题（可空，默认取正文首行）" />
      <div class="grid grid-cols-2 gap-3">
        <SelectField
          :model-value="type"
          :options="typeOptions"
          :clearable="false"
          aria-label="类型"
          @update:model-value="selectType"
        />
        <SelectField
          :model-value="space"
          :options="spaceOptions"
          :clearable="false"
          aria-label="空间"
          @update:model-value="selectSpace"
        />
      </div>
      <TextArea v-model="body" :rows="5" class="flex-1" placeholder="正文，可空；可多行" />
    </div>

    <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>

    <footer class="flex items-center justify-end gap-2">
      <Button type="button" variant="outline" @click="hide">取消</Button>
      <Button type="submit" :disabled="submitting">
        {{ submitting ? '保存中…' : '存入收件箱' }}
      </Button>
    </footer>
  </form>
</template>
