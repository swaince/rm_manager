<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import ItemCreateDialog from '@/components/item/ItemCreateDialog.vue'
import { ITEM_TYPE_LABELS, STATUS_LABELS, SPACE_LABELS, listItems, type Item } from '@/apis/items'

const items = ref<Item[]>([])
const loading = ref(true)
const errorMessage = ref('')

async function load() {
  loading.value = true
  errorMessage.value = ''
  try {
    items.value = await listItems()
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<template>
  <main class="mx-auto flex min-h-screen w-full max-w-3xl flex-col gap-6 p-8">
    <header class="flex items-start justify-between gap-4">
      <div>
        <h1 class="text-xl font-semibold">条目</h1>
        <p class="text-muted-foreground text-sm">
          数据真源是 vault 里的 Markdown 文件，一条目一文件。
        </p>
      </div>
      <ItemCreateDialog @created="load" />
    </header>

    <p v-if="loading" class="text-muted-foreground text-sm">加载中…</p>

    <div v-else-if="errorMessage" class="space-y-3">
      <p class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>
      <Button variant="outline" @click="load">重试</Button>
    </div>

    <Card v-else-if="items.length === 0">
      <CardContent>
        <Empty>
          <EmptyHeader>
            <EmptyTitle>还没有条目</EmptyTitle>
            <EmptyDescription>点击「新建条目」写下第一条想法。</EmptyDescription>
          </EmptyHeader>
        </Empty>
      </CardContent>
    </Card>

    <ul v-else class="space-y-2">
      <li v-for="item in items" :key="item.id">
        <Card>
          <CardContent class="flex flex-col gap-1">
            <span class="font-medium">{{ item.title }}</span>
            <span class="text-muted-foreground text-xs">
              {{ item.id }} · {{ ITEM_TYPE_LABELS[item.type] }} · {{ SPACE_LABELS[item.space] }} ·
              {{ STATUS_LABELS[item.status] }} · {{ item.created.slice(0, 10) }}
            </span>
          </CardContent>
        </Card>
      </li>
    </ul>
  </main>
</template>
