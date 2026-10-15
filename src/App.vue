<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { emit } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import CaptureWindow from '@/views/CaptureWindow.vue'
import InboxTriageView from '@/views/InboxTriageView.vue'
import ItemListView from '@/views/ItemListView.vue'
import PlanningView from '@/views/PlanningView.vue'
import SyncCenter from '@/components/sync/SyncCenter.vue'
import CommandPalette from '@/components/command/CommandPalette.vue'
import ItemCreateDialog from '@/components/item/ItemCreateDialog.vue'
import ItemEditDialog from '@/components/item/ItemEditDialog.vue'
import type { Item } from '@/apis/items'
import { ITEMS_CHANGED, openCaptureWindow, registerCaptureShortcuts } from '@/lib/capture'

/** 迷你速记窗复用同一 index.html：?capture=1 打开，窗口 label 兜底 */
const captureMode =
  new URLSearchParams(window.location.search).get('capture') === '1' ||
  getCurrentWebviewWindow().label === 'capture'

const tab = ref('items')
/** 命令面板与它触发的对话框：主窗全局一处挂载，速记窗不渲染 */
const paletteOpen = ref(false)
const createOpen = ref(false)
const editOpen = ref(false)
const editTarget = ref<Item | null>(null)

function openItemEditor(item: Item) {
  editTarget.value = item
  editOpen.value = true
}

/** 面板 / 它触发的编辑落盘后广播，让条目列表等视图重新查询 */
function notifyItemsChanged() {
  void emit(ITEMS_CHANGED)
}

onMounted(() => {
  // 全局快捷键只在主窗注册一次
  if (!captureMode) void registerCaptureShortcuts()
})
</script>

<template>
  <CaptureWindow v-if="captureMode" />
  <Tabs v-else v-model="tab">
    <header
      class="mx-auto flex w-full max-w-7xl flex-wrap items-center justify-between gap-3 px-8 pt-6"
    >
      <TabsList>
        <TabsTrigger value="items">条目</TabsTrigger>
        <TabsTrigger value="inbox">收件箱</TabsTrigger>
        <TabsTrigger value="planning">规划</TabsTrigger>
      </TabsList>
      <div class="flex flex-wrap items-center gap-2">
        <SyncCenter />
        <Button variant="outline" size="sm" @click="openCaptureWindow">
          速记（Ctrl+Shift+Space）
        </Button>
      </div>
    </header>
    <TabsContent value="items">
      <ItemListView />
    </TabsContent>
    <TabsContent value="inbox">
      <InboxTriageView />
    </TabsContent>
    <TabsContent value="planning">
      <PlanningView />
    </TabsContent>
  </Tabs>
  <CommandPalette
    v-if="!captureMode"
    v-model:open="paletteOpen"
    @navigate="(next) => (tab = next)"
    @edit="openItemEditor"
    @create="createOpen = true"
  />
  <ItemCreateDialog
    v-if="!captureMode"
    :open="createOpen"
    @update:open="(value) => (createOpen = value)"
    @created="notifyItemsChanged"
  />
  <ItemEditDialog
    v-if="editTarget"
    :item="editTarget"
    :open="editOpen"
    @update:open="(value) => (editOpen = value)"
    @updated="notifyItemsChanged"
  />
</template>
