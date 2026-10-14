<script setup lang="ts">
import { ref } from 'vue'
import { Button } from '@/components/ui/button'
import ItemListView from '@/views/ItemListView.vue'
import PlanningView from '@/views/PlanningView.vue'

/** 顶层导航：条目（列表 / 搜索）与规划（需求池 / 排期 / 今日焦点）两个视图 */
const tabs = [
  { value: 'items', label: '条目' },
  { value: 'planning', label: '规划' },
] as const

const tab = ref<'items' | 'planning'>('items')
</script>

<template>
  <div>
    <nav class="mx-auto flex w-full max-w-5xl items-center gap-2 px-8 pt-6" aria-label="主导航">
      <Button
        v-for="item in tabs"
        :key="item.value"
        size="sm"
        :variant="tab === item.value ? 'default' : 'ghost'"
        @click="tab = item.value"
      >
        {{ item.label }}
      </Button>
    </nav>
    <ItemListView v-if="tab === 'items'" />
    <PlanningView v-else />
  </div>
</template>
