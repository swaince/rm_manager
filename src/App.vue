<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

const greetMsg = ref('')
const name = ref('')

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke('greet', { name: name.value })
}
</script>

<template>
  <main class="flex min-h-screen items-center justify-center p-8">
    <Card class="w-full max-w-md">
      <CardHeader>
        <CardTitle>rm-manager</CardTitle>
        <CardDescription>个人需求管理与规划工具 · 骨架已就绪</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="space-y-2">
          <Label for="greet-input">试一句问候（验证前后端通道）</Label>
          <Input id="greet-input" v-model="name" placeholder="输入名字..." @keyup.enter="greet" />
        </div>
        <Button class="w-full" @click="greet">问候</Button>
        <p v-if="greetMsg" class="text-muted-foreground text-sm">{{ greetMsg }}</p>
      </CardContent>
    </Card>
  </main>
</template>
