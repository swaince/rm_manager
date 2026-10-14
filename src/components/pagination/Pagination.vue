<script setup lang="ts">
import type { PropType } from 'vue'
import { computed, ref, useId, watch } from 'vue'
import {
  PaginationItem,
  PaginationEllipsis,
  PaginationContent as PaginationList,
  PaginationNext,
  PaginationPrevious as PaginationPrev,
  Pagination as PaginationRoot,
} from '@/components/ui/pagination'

import { SelectField, TextInput } from '@/components/form'
import type { SelectOption } from '@/components/form'
import { ChevronLeft, ChevronRight } from '@lucide/vue'
defineOptions({ name: 'PhoenixPagination' })

const defaultPageSizeOptions = [10, 20, 30, 50, 100] as const
const defaultPageSize = 20

function isPositiveInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) > 0
}

function isNonNegativeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) >= 0
}

function isValidPageSizeOptions(value: unknown): value is readonly number[] {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    value.every(isPositiveInteger) &&
    new Set(value).size === value.length
  )
}

function normalizePageSize(pageSize: unknown, pageSizeOptions: unknown): number {
  const options = isValidPageSizeOptions(pageSizeOptions) ? pageSizeOptions : defaultPageSizeOptions
  const requestedSize = isPositiveInteger(pageSize) ? pageSize : defaultPageSize

  return options.includes(requestedSize) ? requestedSize : (options[0] ?? defaultPageSize)
}

const props = defineProps({
  page: {
    type: Number,
    default: 1,
  },
  pageSize: {
    type: Number,
    default: 20,
  },
  total: {
    type: Number,
    default: 0,
  },
  pageSizeOptions: {
    type: Array as PropType<readonly number[]>,
    default: () => [10, 20, 30, 50, 100],
  },
  disabled: {
    type: Boolean,
    default: false,
  },
})

const emit = defineEmits<{
  'update:page': [page: number]
  'update:pageSize': [pageSize: number]
}>()

const safePage = computed(() => (isPositiveInteger(props.page) ? props.page : 1))
const safePageSize = computed(() => normalizePageSize(props.pageSize, props.pageSizeOptions))
const safeTotal = computed(() => (isNonNegativeInteger(props.total) ? props.total : 0))
const safePageSizeOptions = computed(() =>
  isValidPageSizeOptions(props.pageSizeOptions) ? props.pageSizeOptions : defaultPageSizeOptions,
)
const pageCount = computed(() => Math.ceil(safeTotal.value / safePageSize.value))
const normalizedPage = computed(() =>
  pageCount.value === 0 ? 1 : Math.min(safePage.value, pageCount.value),
)
const navigationDisabled = computed(() => props.disabled || safeTotal.value === 0)
const pageSizeSelectOptions = computed<SelectOption[]>(() =>
  safePageSizeOptions.value.map((pageSize) => ({
    value: String(pageSize),
    label: `${pageSize}条`,
  })),
)

const jumpInput = ref(String(normalizedPage.value))
const jumpError = ref('')
const jumpErrorId = `pagination-jump-error-${useId()}`

watch(
  () => [props.page, props.pageSize, props.total, props.pageSizeOptions] as const,
  ([page, pageSize, total, pageSizeOptions], previous) => {
    if (!isPositiveInteger(page)) {
      console.warn('[Pagination] page must be a positive integer.')
    }
    if (!isPositiveInteger(pageSize)) {
      console.warn('[Pagination] pageSize must be a positive integer.')
    }
    if (!isNonNegativeInteger(total)) {
      console.warn('[Pagination] total must be a non-negative integer.')
    }
    if (!isValidPageSizeOptions(pageSizeOptions)) {
      console.warn('[Pagination] pageSizeOptions must contain distinct positive integers.')
    }

    const nextPageSize = normalizePageSize(pageSize, pageSizeOptions)
    if (isPositiveInteger(pageSize) && pageSize !== nextPageSize) {
      emit('update:pageSize', nextPageSize)
    }

    const pageSizeChanged =
      previous !== undefined && normalizePageSize(previous[1], previous[3]) !== nextPageSize
    const nextPage = pageSizeChanged ? 1 : normalizedPage.value
    if (page !== nextPage) emit('update:page', nextPage)
  },
  { immediate: true, deep: true, flush: 'sync' },
)

watch(normalizedPage, (page) => {
  jumpInput.value = String(page)
  jumpError.value = ''
})

function navigateTo(page: number) {
  if (navigationDisabled.value || !isPositiveInteger(page) || page > pageCount.value) return
  emit('update:page', page)
}

function updatePageSize(value: string) {
  const pageSize = Number(value)
  if (
    props.disabled ||
    !isPositiveInteger(pageSize) ||
    !safePageSizeOptions.value.includes(pageSize)
  ) {
    return
  }

  emit('update:pageSize', pageSize)
  emit('update:page', 1)
  jumpInput.value = '1'
  jumpError.value = ''
}

function updateJumpInput(value: string) {
  jumpInput.value = value
  jumpError.value = ''
}

function submitJump() {
  if (navigationDisabled.value) return

  const value = jumpInput.value.trim()
  if (!/^[1-9]\d*$/.test(value)) {
    jumpError.value = `请输入 1 到 ${pageCount.value} 之间的整数页码。`
    return
  }

  const page = Number(value)
  if (!isPositiveInteger(page) || page > pageCount.value) {
    jumpError.value = `请输入 1 到 ${pageCount.value} 之间的整数页码。`
    return
  }

  jumpError.value = ''
  if (page !== normalizedPage.value) navigateTo(page)
}
</script>

<template>
  <PaginationRoot
    v-slot="{ page }"
    :page="normalizedPage"
    :items-per-page="safePageSize"
    :total="safeTotal"
    :disabled="navigationDisabled"
    :sibling-count="1"
    show-edges
    aria-label="分页"
    class="flex w-full min-w-0 flex-wrap items-center justify-end gap-x-4 gap-y-3"
    @update:page="navigateTo"
  >
    <span class="me-auto whitespace-nowrap text-sm text-muted-foreground tabular-nums">
      共 {{ safeTotal }} 条
    </span>
    <PaginationList v-slot="{ items }" class="flex min-w-0 flex-wrap items-center gap-1">
      <PaginationPrev aria-label="上一页" size="sm">
        <ChevronLeft class="size-4" aria-hidden="true" />
        <span>上一页</span>
      </PaginationPrev>
      <template v-for="(item, index) in pageCount ? items : []" :key="index">
        <PaginationItem
          v-if="item.type === 'page'"
          :value="item.value"
          :is-active="item.value === page"
          :aria-label="`第 ${item.value} 页`"
          size="icon-sm"
        >
          {{ item.value }}
        </PaginationItem>
        <PaginationEllipsis v-else :index="index" class="size-8" />
      </template>
      <PaginationNext aria-label="下一页" size="sm">
        <span>下一页</span>
        <ChevronRight class="size-4" aria-hidden="true" />
      </PaginationNext>
    </PaginationList>

    <div class="w-36 shrink-0">
      <SelectField
        aria-label="每页条数"
        :model-value="String(safePageSize)"
        :options="pageSizeSelectOptions"
        :clearable="false"
        :disabled="props.disabled"
        size="sm"
        @update:model-value="updatePageSize"
      />
    </div>

    <div class="flex items-center gap-2 text-sm text-muted-foreground">
      <span>到</span>
      <div class="w-14 shrink-0">
        <TextInput
          aria-label="跳转到页码"
          :described-by="jumpError ? jumpErrorId : undefined"
          :model-value="jumpInput"
          :disabled="navigationDisabled"
          :invalid="Boolean(jumpError)"
          inputmode="numeric"
          autocomplete="off"
          class="h-8 text-center tabular-nums"
          @update:model-value="updateJumpInput"
          @keydown.enter.prevent="submitJump"
          @blur="submitJump"
        />
      </div>
      <span class="whitespace-nowrap">/ {{ pageCount }} 页</span>
    </div>
    <p
      v-if="jumpError"
      :id="jumpErrorId"
      role="alert"
      class="basis-full text-end text-sm text-destructive"
    >
      {{ jumpError }}
    </p>
  </PaginationRoot>
</template>
