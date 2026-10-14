<script setup lang="ts">
import { computed, ref } from 'vue'
import { useElementSize } from '@vueuse/core'
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area'

import { InboxIcon } from '@lucide/vue'
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty'
import { Spinner } from '@/components/ui/spinner'

import {
  Table,
  TableBody,
  TableCell,
  TableEmpty,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'

import type { DataTableColumn, DataTableRow } from './data-table'

const props = withDefaults(
  defineProps<{
    columns: DataTableColumn[]
    rows: DataTableRow[]
    rowKey?: string
    height?: number | string
    loading?: boolean
    emptyText?: string
    emptyDescription?: string
  }>(),
  {
    rowKey: 'id',
    height: 320,
    loading: false,
    emptyText: '暂无数据',
    emptyDescription: '',
  },
)

const tableHeight = computed(() =>
  typeof props.height === 'number' ? `${Math.max(0, props.height)}px` : props.height,
)
const isTableState = computed(() => props.loading || props.rows.length === 0)
const tableHeader = ref<InstanceType<typeof TableHeader>>()
const { height: headerHeight } = useElementSize(computed(() => tableHeader.value?.$el))

function keyFor(row: DataTableRow, index: number): string | number {
  const key = row[props.rowKey]

  return typeof key === 'string' || typeof key === 'number' ? key : index
}

function displayValue(value: unknown): string {
  return value === null || value === undefined ? '' : String(value)
}
</script>

<template>
  <ScrollArea
    class="data-table-scroll overflow-hidden rounded-md border border-border bg-card"
    :style="{ height: tableHeight, '--table-header-height': `${headerHeight}px` }"
    :aria-busy="props.loading"
  >
    <Table :class="['min-w-full border-separate border-spacing-0', isTableState && 'h-full']">
      <colgroup>
        <col
          v-for="column in columns"
          :key="column.key"
          :style="column.width ? { width: column.width } : undefined"
        />
      </colgroup>
      <TableHeader ref="tableHeader" :class="{ 'h-0': isTableState }">
        <TableRow>
          <TableHead
            v-for="column in columns"
            :key="column.key"
            scope="col"
            class="sticky top-0 z-10 bg-muted"
            :style="{ textAlign: column.align ?? 'left' }"
          >
            {{ column.label }}
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody :class="{ 'h-full': isTableState }">
        <TableEmpty
          v-if="isTableState"
          :colspan="Math.max(props.columns.length, 1)"
          class="h-full whitespace-normal bg-muted/50 p-0 [&>div]:h-full [&>div]:py-0"
        >
          <Empty
            role="status"
            aria-live="polite"
            class="h-full w-full gap-2 rounded-none p-4 [&_[data-slot=empty-description]]:text-xs"
          >
            <EmptyHeader class="gap-2">
              <EmptyMedia class="mb-0">
                <Spinner
                  v-if="props.loading"
                  class="size-6"
                  role="presentation"
                  aria-hidden="true"
                />
                <InboxIcon v-else class="size-6 text-muted-foreground" aria-hidden="true" />
              </EmptyMedia>
              <EmptyTitle class="text-sm font-medium tracking-normal">
                {{ props.loading ? '加载中…' : props.emptyText }}
              </EmptyTitle>
              <EmptyDescription v-if="props.loading || props.emptyDescription">
                {{ props.loading ? '正在加载数据，请稍候。' : props.emptyDescription }}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        </TableEmpty>
        <TableRow v-else v-for="(row, index) in props.rows" :key="keyFor(row, index)">
          <TableCell
            v-for="column in columns"
            :key="column.key"
            :style="{ textAlign: column.align ?? 'left' }"
          >
            <slot :name="`cell-${column.key}`" :row="row" :value="row[column.key]" :column="column">
              {{ displayValue(row[column.key]) }}
            </slot>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
    <ScrollBar orientation="horizontal" />
  </ScrollArea>
</template>

<style scoped>
.data-table-scroll :deep([data-slot='scroll-area-viewport'] > div) {
  height: 100%;
}

.data-table-scroll :deep([data-slot='table-container']) {
  height: 100%;
  overflow: visible;
}

.data-table-scroll :deep([data-slot='scroll-area-scrollbar'][data-orientation='vertical']) {
  top: var(--table-header-height) !important;
  height: auto;
}
</style>
