<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { ChevronsLeft, ChevronsRight } from '@lucide/vue'
import type { DateValue } from 'reka-ui'
import { Button } from '@/components/ui/button'
import { Calendar, CalendarPrevButton, CalendarNextButton } from '@/components/ui/calendar'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

interface Props {
  modelValue?: DateValue
  placeholder: DateValue
  minValue?: DateValue
  maxValue?: DateValue
  isDateDisabled?: (date: DateValue) => boolean
  disabled?: boolean
  readonly?: boolean
  initialFocus?: boolean
  preventDeselect?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: undefined,
  minValue: undefined,
  maxValue: undefined,
  isDateDisabled: undefined,
  disabled: false,
  readonly: false,
  initialFocus: false,
  preventDeselect: false,
  class: undefined,
})

const emit = defineEmits<{
  'update:modelValue': [value: DateValue | undefined]
}>()

const visibleDate = shallowRef(clampDate(props.placeholder))
const invalidRange = computed(() =>
  Boolean(props.minValue && props.maxValue && props.minValue.compare(props.maxValue) > 0),
)
const yearOptions = computed(() => {
  if (invalidRange.value) return []

  const first = Math.max(1, props.minValue?.year ?? visibleDate.value.year - 100)
  const last = Math.min(9999, props.maxValue?.year ?? visibleDate.value.year + 10)
  return last < first ? [] : Array.from({ length: last - first + 1 }, (_, index) => first + index)
})
const monthOptions = computed(() => {
  const year = visibleDate.value.year
  const first = props.minValue?.year === year ? props.minValue.month : 1
  const last = props.maxValue?.year === year ? props.maxValue.month : 12
  return Array.from({ length: Math.max(0, last - first + 1) }, (_, index) => first + index)
})
const monthLabel = (month: number) => `${month}月`

watch(
  () => [props.placeholder, props.minValue, props.maxValue] as const,
  ([placeholder]) => {
    visibleDate.value = clampDate(placeholder)
  },
)

function clampDate(date: DateValue): DateValue {
  if (props.minValue && date.compare(props.minValue) < 0) return props.minValue
  if (props.maxValue && date.compare(props.maxValue) > 0) return props.maxValue
  return date
}

function monthBounds(year: number) {
  return {
    first: props.minValue?.year === year ? props.minValue.month : 1,
    last: props.maxValue?.year === year ? props.maxValue.month : 12,
  }
}

function setYear(value: unknown) {
  const year = typeof value === 'string' ? Number(value) : Number.NaN
  if (props.disabled || !yearOptions.value.includes(year)) return

  const { first, last } = monthBounds(year)
  const month = Math.min(last, Math.max(first, visibleDate.value.month))
  visibleDate.value = visibleDate.value.set({ year, month, day: 1 })
}

function setMonth(value: unknown) {
  const month = typeof value === 'string' ? Number(value) : Number.NaN
  if (props.disabled || !monthOptions.value.includes(month)) return
  visibleDate.value = visibleDate.value.set({ month, day: 1 })
}

function canNavigateToYear(year: number): boolean {
  return (
    !props.disabled &&
    !invalidRange.value &&
    year >= 1 &&
    year <= 9999 &&
    (!props.minValue || year >= props.minValue.year) &&
    (!props.maxValue || year <= props.maxValue.year)
  )
}
</script>

<template>
  <Calendar
    v-model:placeholder="visibleDate"
    :model-value="modelValue"
    :min-value="minValue"
    :max-value="maxValue"
    :is-date-disabled="isDateDisabled"
    :disabled="disabled"
    :readonly="readonly"
    :initial-focus="initialFocus"
    :prevent-deselect="preventDeselect"
    :class="props.class"
    class="[&_[data-slot=calendar-header]]:px-0 [&_[data-slot=calendar-header]>nav]:hidden"
    locale="zh-CN"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template #calendar-heading>
      <TooltipProvider disable-hoverable-content>
        <div class="flex w-full min-w-0 items-center gap-1">
          <Tooltip>
            <TooltipTrigger as-child>
              <Button
                type="button"
                variant="outline"
                size="icon-xs"
                class="size-7 shrink-0 bg-transparent p-0 opacity-50 hover:opacity-100"
                aria-label="上一年"
                :disabled="!canNavigateToYear(visibleDate.year - 1)"
                @click="setYear(String(visibleDate.year - 1))"
              >
                <ChevronsLeft class="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>上一年</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger as-child>
              <CalendarPrevButton class="shrink-0" aria-label="上一月" />
            </TooltipTrigger>
            <TooltipContent>上一月</TooltipContent>
          </Tooltip>
          <Select
            :model-value="String(visibleDate.year)"
            :disabled="disabled || yearOptions.length === 0"
            @update:model-value="setYear"
          >
            <SelectTrigger size="sm" class="min-w-20 flex-1 px-2 text-xs" aria-label="选择年份">
              <SelectValue :placeholder="`${visibleDate.year}年`" />
            </SelectTrigger>
            <SelectContent position="popper" align="center" class="max-h-60">
              <SelectItem v-for="year in yearOptions" :key="year" :value="String(year)">
                {{ year }}年
              </SelectItem>
            </SelectContent>
          </Select>
          <Select
            :model-value="String(visibleDate.month)"
            :disabled="disabled || monthOptions.length === 0"
            @update:model-value="setMonth"
          >
            <SelectTrigger size="sm" class="min-w-16 flex-1 px-2 text-xs" aria-label="选择月份">
              <SelectValue :placeholder="monthLabel(visibleDate.month)" />
            </SelectTrigger>
            <SelectContent position="popper" align="center" class="max-h-60">
              <SelectItem v-for="month in monthOptions" :key="month" :value="String(month)">
                {{ monthLabel(month) }}
              </SelectItem>
            </SelectContent>
          </Select>
          <Tooltip>
            <TooltipTrigger as-child>
              <CalendarNextButton class="shrink-0" aria-label="下一月" />
            </TooltipTrigger>
            <TooltipContent>下一月</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger as-child>
              <Button
                type="button"
                variant="outline"
                size="icon-xs"
                class="size-7 shrink-0 bg-transparent p-0 opacity-50 hover:opacity-100"
                aria-label="下一年"
                :disabled="!canNavigateToYear(visibleDate.year + 1)"
                @click="setYear(String(visibleDate.year + 1))"
              >
                <ChevronsRight class="size-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>下一年</TooltipContent>
          </Tooltip>
        </div>
      </TooltipProvider>
    </template>
  </Calendar>
</template>
