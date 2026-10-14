<script setup lang="ts">
import { ref, watch } from 'vue'
import { emit as emitTauri } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogScrollContent,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  continueSync,
  resolveConflict,
  syncConflicts,
  type ConflictEntry,
  type ConflictResolution,
  type SyncStatus,
} from '@/apis/sync'
import { ITEMS_CHANGED } from '@/lib/capture'

/** 冲突解决：逐条三选一（保留双方 / 用本地 / 用远端），全部解决后继续同步（FR-9.2） */
const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{
  status: [status: SyncStatus]
}>()

const conflicts = ref<ConflictEntry[]>([])
const busy = ref(false)
const errorMessage = ref('')

watch(open, (value) => {
  if (value) void load()
})

async function load() {
  errorMessage.value = ''
  try {
    conflicts.value = await syncConflicts()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  }
}

async function resolve(entry: ConflictEntry, resolution: ConflictResolution) {
  if (busy.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    await resolveConflict(entry.path, resolution)
    conflicts.value = conflicts.value.filter((candidate) => candidate.path !== entry.path)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    busy.value = false
  }
}

async function finish() {
  if (busy.value) return
  busy.value = true
  errorMessage.value = ''
  try {
    const status = await continueSync()
    emit('status', status)
    if (status.state === 'conflict') {
      // rebase 里还有后续提交又撞了冲突：刷新列表接着解决
      await load()
      return
    }
    void emitTauri(ITEMS_CHANGED)
    open.value = false
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogScrollContent class="max-w-3xl">
      <DialogHeader>
        <DialogTitle>冲突解决</DialogTitle>
        <DialogDescription>
          两端改了同一条目同一处，git 不能自动合并。逐条选择保留哪一侧；「保留双方」会把远端版本
          另存为副本条目——任何一侧的内容都不会丢。
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-3">
        <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>
        <p v-if="!conflicts.length" class="text-muted-foreground text-sm" role="status">
          全部冲突已解决，点「继续同步」完成 rebase 并推送。
        </p>

        <div
          v-for="entry in conflicts"
          :key="entry.path"
          class="space-y-2 rounded-md border border-border p-3"
        >
          <div>
            <p class="text-sm font-medium">{{ entry.title ?? entry.path }}</p>
            <p class="text-muted-foreground text-xs">{{ entry.item_id ?? entry.path }}</p>
          </div>
          <div class="grid gap-2 sm:grid-cols-2">
            <div class="min-w-0">
              <p class="text-xs font-medium">本地版本</p>
              <pre
                class="bg-muted mt-1 max-h-32 overflow-auto rounded p-2 text-xs whitespace-pre-wrap"
                >{{ entry.local }}</pre>
            </div>
            <div class="min-w-0">
              <p class="text-xs font-medium">远端版本</p>
              <pre
                class="bg-muted mt-1 max-h-32 overflow-auto rounded p-2 text-xs whitespace-pre-wrap"
                >{{ entry.remote }}</pre>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              :disabled="busy"
              @click="resolve(entry, 'keep_both')"
            >
              保留双方（远端存为副本）
            </Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="busy"
              @click="resolve(entry, 'use_local')"
            >
              用本地
            </Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="busy"
              @click="resolve(entry, 'use_remote')"
            >
              用远端
            </Button>
          </div>
        </div>
      </div>

      <DialogFooter class="gap-2">
        <p class="text-muted-foreground mr-auto text-xs">
          {{ conflicts.length ? `还剩 ${conflicts.length} 处待解决` : '可以继续同步' }}
        </p>
        <Button :disabled="busy || conflicts.length > 0" @click="finish">继续同步</Button>
      </DialogFooter>
    </DialogScrollContent>
  </Dialog>
</template>
