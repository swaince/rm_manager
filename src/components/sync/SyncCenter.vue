<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { emit, listen } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import ConflictDialog from './ConflictDialog.vue'
import SyncSettingsDialog from './SyncSettingsDialog.vue'
import { syncNow, syncStatus, type SyncStatus } from '@/apis/sync'
import { ITEMS_CHANGED } from '@/lib/capture'
import { SYNC_STATUS, syncStateDotClass, syncStateText } from '@/lib/sync'

/** 同步中心：状态角标 + 设置对话框 + 冲突对话框；状态变化听 Rust 侧广播的 sync-status */
const busy = ref(false)
const status = ref<SyncStatus | null>(null)
const settingsOpen = ref(false)
const conflictOpen = ref(false)

const label = computed(() => (status.value ? syncStateText(status.value) : '同步'))
const dotClass = computed(() =>
  status.value ? syncStateDotClass(status.value) : 'bg-muted-foreground',
)

/** 结果落到角标：冲突自动弹解决对话框，拉到新提交则刷新条目列表 */
function apply(next: SyncStatus) {
  status.value = next
  if (next.state === 'conflict') conflictOpen.value = true
  if (next.pulled > 0) void emit(ITEMS_CHANGED)
}

async function refresh() {
  try {
    status.value = await syncStatus()
  } catch {
    // 状态查询失败不打扰使用：角标保持上一次的值
  }
}

async function runSync() {
  if (busy.value) return
  busy.value = true
  try {
    apply(await syncNow())
  } catch {
    // 网络 / 认证 / 冲突的失败分类在 Rust 返回的状态里，命令本身的错误忽略
  } finally {
    busy.value = false
    await refresh()
  }
}

/** 角标点击：冲突先去解决，未配置 / 认证失败去设置，其余同步一次 */
function onBadgeClick() {
  if (status.value?.state === 'conflict') {
    conflictOpen.value = true
    return
  }
  if (
    !status.value ||
    status.value.state === 'not_configured' ||
    status.value.last_error?.kind === 'auth'
  ) {
    settingsOpen.value = true
    return
  }
  void runSync()
}

let unlisten: (() => void) | undefined

onMounted(async () => {
  await refresh()
  unlisten = await listen<SyncStatus>(SYNC_STATUS, (event) => apply(event.payload))
})

onUnmounted(() => unlisten?.())
</script>

<template>
  <div class="flex items-center gap-2">
    <Button variant="outline" size="sm" :disabled="busy" @click="onBadgeClick">
      <span class="size-2 rounded-full" :class="dotClass" aria-hidden="true" />
      {{ busy ? '同步中…' : label }}
    </Button>
    <Button variant="ghost" size="sm" @click="settingsOpen = true">同步设置</Button>
    <SyncSettingsDialog v-model:open="settingsOpen" :status="status" @updated="apply" />
    <ConflictDialog v-model:open="conflictOpen" @status="apply" />
  </div>
</template>
