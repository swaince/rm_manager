<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { FormField, TextInput } from '@/components/form'
import { setRemote, testRemote, type SyncFailure, type SyncStatus } from '@/apis/sync'
import { syncFailureHint, syncStateText } from '@/lib/sync'

/** 同步设置：远端 URL / 用户名 / PAT；保存 = 连接并首次推送（凭据进系统凭据库，不落文件） */
const props = defineProps<{ status: SyncStatus | null }>()
const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{
  updated: [status: SyncStatus]
}>()

const url = ref('')
const user = ref('')
const token = ref('')
const busy = ref(false)
const failure = ref<SyncFailure | null>(null)
const notice = ref('')

// 文案常量：模板静态属性里不能直接写 "://"（vue-tsc 会误判插槽作用域），一律绑定常量
const URL_PLACEHOLDER = 'https://gitee.com/用户名/rm-vault.git'
const URL_HINT = 'HTTPS 地址，如 https://gitee.com/用户名/rm-vault.git'

const currentText = computed(() => (props.status ? syncStateText(props.status) : ''))

watch(open, (value) => {
  if (value) {
    failure.value = null
    notice.value = ''
  }
})

function toFailure(error: unknown): SyncFailure {
  return { kind: 'network', message: error instanceof Error ? error.message : String(error) }
}

async function runTest() {
  if (busy.value || !url.value.trim()) return
  busy.value = true
  failure.value = null
  notice.value = ''
  try {
    const result = await testRemote(url.value, user.value, token.value)
    if (result) failure.value = result
    else notice.value = '连接成功：远端可访问。'
  } catch (error) {
    failure.value = toFailure(error)
  } finally {
    busy.value = false
  }
}

async function save() {
  if (busy.value || !url.value.trim()) return
  busy.value = true
  failure.value = null
  notice.value = ''
  try {
    const status = await setRemote(url.value, user.value, token.value)
    emit('updated', status)
    if (status.last_error) {
      // 远端已配置，但首次推送没成：按分类给下一步提示，留在对话框里改
      failure.value = status.last_error
      return
    }
    open.value = false
  } catch (error) {
    failure.value = toFailure(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-xl">
      <DialogHeader>
        <DialogTitle>同步设置</DialogTitle>
        <DialogDescription>
          连接 Gitee / GitHub 私有仓库。访问令牌只存 Windows 凭据管理器，调用 git
          时临时注入，不写进任何配置文件。
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        <p v-if="currentText" class="text-muted-foreground text-sm">当前状态：{{ currentText }}</p>

        <FormField label="远端地址" required :hint="URL_HINT">
          <template #default="{ id, invalid, describedBy, required }">
            <TextInput
              :id="id"
              v-model="url"
              :invalid="invalid"
              :described-by="describedBy"
              :required="required"
              :placeholder="URL_PLACEHOLDER"
            />
          </template>
        </FormField>

        <FormField label="用户名" hint="Gitee 必填（登录名）；GitHub 可留空">
          <template #default="{ id, describedBy }">
            <TextInput
              :id="id"
              v-model="user"
              :described-by="describedBy"
              placeholder="Gitee 登录名"
            />
          </template>
        </FormField>

        <FormField label="访问令牌（PAT）" required hint="最小权限即可：只授权该仓库的读 / 写">
          <template #default="{ id, invalid, describedBy, required }">
            <TextInput
              :id="id"
              v-model="token"
              type="password"
              :invalid="invalid"
              :described-by="describedBy"
              :required="required"
              placeholder="粘贴私人令牌"
            />
          </template>
        </FormField>

        <p v-if="notice" class="text-sm" role="status">{{ notice }}</p>
        <div v-if="failure" class="space-y-1" role="alert">
          <p class="text-destructive text-sm">{{ syncFailureHint(failure) }}</p>
          <p class="text-muted-foreground text-xs break-all">{{ failure.message }}</p>
        </div>
      </div>

      <DialogFooter class="gap-2">
        <Button variant="outline" :disabled="busy || !url.trim()" @click="runTest">测试连接</Button>
        <Button :disabled="busy || !url.trim()" @click="save">保存并首次推送</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
