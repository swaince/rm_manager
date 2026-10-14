<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import CaptureWindow from '@/views/CaptureWindow.vue'
import InboxTriageView from '@/views/InboxTriageView.vue'
import ItemListView from '@/views/ItemListView.vue'
import PlanningView from '@/views/PlanningView.vue'
import { openCaptureWindow, registerCaptureShortcuts } from '@/lib/capture'

/** 迷你速记窗复用同一 index.html：?capture=1 打开，窗口 label 兜底 */
const captureMode =
  new URLSearchParams(window.location.search).get('capture') === '1' ||
  getCurrentWebviewWindow().label === 'capture'

const tab = ref('items')

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
      <Button variant="outline" size="sm" @click="openCaptureWindow">
        速记（Ctrl+Shift+Space）
      </Button>
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
</template>
</template>
