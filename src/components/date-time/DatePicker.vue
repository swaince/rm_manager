<script setup lang="ts">
import { getLocalTimeZone, today } from '@internationalized/date'
import type { DateValue } from '@internationalized/date'
import { CalendarDays, X } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import DateCalendar from './DateCalendar.vue'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { isDateWithinRange, parseCalendarDate } from './date-time'
import type { DatePickerProps } from './types'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<DatePickerProps>(), {
  modelValue: '',
  disabled: false,
  readonly: false,
  invalid: false,
  describedBy: undefined,
  ariaLabel: undefined,
  ariaLabelledby: undefined,
  placeholder: '选择日期',
  clearable: false,
  min: undefined,
  max: undefined,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const isOpen = ref(false)
const selectedDate = computed(() => parseCalendarDate(props.modelValue))
const minValue = computed(() => parseCalendarDate(props.min))
const maxValue = computed(() => parseCalendarDate(props.max))
const calendarPlaceholder = computed(
  () => selectedDate.value ?? minValue.value ?? today(getLocalTimeZone()),
)
const displayValue = computed(
  () => selectedDate.value?.toString() ?? (props.modelValue || props.placeholder),
)
const invalidRange = computed(() => {
  if ((props.min && !minValue.value) || (props.max && !maxValue.value)) return true
  return Boolean(minValue.value && maxValue.value && minValue.value.compare(maxValue.value) > 0)
})
const isInvalid = computed(() => {
  if (props.invalid || invalidRange.value) return true
  if (!props.modelValue) return false
  return (
    !selectedDate.value || !isDateWithinRange(selectedDate.value, minValue.value, maxValue.value)
  )
})

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) isOpen.value = false
  },
)

function isOutOfRange(date: DateValue): boolean {
  if (invalidRange.value) return true
  if (minValue.value && date.compare(minValue.value) < 0) return true
  if (maxValue.value && date.compare(maxValue.value) > 0) return true
  return false
}

function updateDate(date?: DateValue) {
  if (props.disabled || props.readonly || !date || isOutOfRange(date)) return
  emit('update:modelValue', date.toString())
  isOpen.value = false
}

function clearDate() {
  if (!props.disabled && !props.readonly) emit('update:modelValue', '')
}
</script>

<template>
  <Popover v-model:open="isOpen">
    <div class="relative w-full">
      <PopoverTrigger as-child>
        <Button
          :id="id"
          type="button"
          variant="outline"
          class="form-control-surface min-w-0 w-full justify-start border-input text-left font-normal"
          :class="clearable && modelValue && !readonly ? 'pr-9' : undefined"
          :disabled="disabled"
          :aria-label="ariaLabel"
          :aria-labelledby="ariaLabelledby"
          :aria-describedby="describedBy"
          :aria-invalid="isInvalid ? 'true' : undefined"
        >
          <CalendarDays class="size-4 shrink-0" />
          <span class="truncate" :class="modelValue ? undefined : 'text-muted-foreground'">
            {{ displayValue }}
          </span>
        </Button>
      </PopoverTrigger>

      <Button
        v-if="clearable && modelValue && !readonly"
        type="button"
        variant="ghost"
        size="icon-xs"
        class="absolute right-1 top-1/2 -translate-y-1/2"
        :disabled="disabled"
        aria-label="清除日期"
        @click="clearDate"
      >
        <X class="size-4" />
      </Button>
    </div>

    <PopoverContent
      align="start"
      aria-label="日期选择器"
      class="w-auto max-h-(--reka-popover-content-available-height) overflow-y-auto p-0"
    >
      <DateCalendar
        class="w-80 max-w-full [&_[data-slot=calendar-cell-trigger]]:size-10"
        :model-value="selectedDate ?? undefined"
        :placeholder="calendarPlaceholder"
        :min-value="minValue"
        :max-value="maxValue"
        :is-date-disabled="isOutOfRange"
        :disabled="disabled"
        :readonly="readonly"
        initial-focus
        :prevent-deselect="true"
        @update:model-value="updateDate"
      />
    </PopoverContent>
  </Popover>
</template>
