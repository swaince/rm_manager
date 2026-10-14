<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Time } from '@internationalized/date'
import { Clock3, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import TimeOptions from './TimeOptions.vue'
import {
  clampTimeToRange,
  formatTimeValue,
  isMinuteWithinRange,
  isTimeWithinRange,
  parseTimeValue,
  timeToSeconds,
} from './date-time'
import type { TimePickerProps } from './types'

const props = withDefaults(defineProps<TimePickerProps>(), {
  modelValue: '',
  disabled: false,
  readonly: false,
  invalid: false,
  describedBy: undefined,
  ariaLabel: undefined,
  ariaLabelledby: undefined,
  name: undefined,
  placeholder: '选择时间',
  clearable: false,
  min: undefined,
  max: undefined,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const isOpen = ref(false)
const selectedTime = computed(() => parseTimeValue(props.modelValue))
const minValue = computed(() => parseTimeValue(props.min))
const maxValue = computed(() => parseTimeValue(props.max))
const invalidRange = computed(() => {
  if ((props.min && !minValue.value) || (props.max && !maxValue.value)) return true
  return Boolean(
    minValue.value &&
    maxValue.value &&
    timeToSeconds(minValue.value) > timeToSeconds(maxValue.value),
  )
})

const isInvalid = computed(() => {
  if (props.invalid || invalidRange.value) return true
  if (!props.modelValue) return false
  return (
    !selectedTime.value || !isTimeWithinRange(selectedTime.value, minValue.value, maxValue.value)
  )
})
const displayValue = computed(() =>
  selectedTime.value ? formatTimeValue(selectedTime.value) : props.modelValue || props.placeholder,
)

const baseTime = computed(() => selectedTime.value ?? minValue.value ?? new Time(0, 0, 0))
const selectedHour = computed(() => String(baseTime.value.hour).padStart(2, '0'))
const selectedMinute = computed(() => String(baseTime.value.minute).padStart(2, '0'))
const selectedSecond = computed(() => String(baseTime.value.second).padStart(2, '0'))
const hourOptions = computed(() =>
  Array.from({ length: 24 }, (_, hour) => {
    const hourStart = hour * 3600
    const hourEnd = hourStart + 3599
    const minSeconds = minValue.value ? timeToSeconds(minValue.value) : 0
    const maxSeconds = maxValue.value ? timeToSeconds(maxValue.value) : 86399
    return {
      value: formatHour(hour),
      disabled: invalidRange.value || hourEnd < minSeconds || hourStart > maxSeconds,
    }
  }),
)
const minuteOptions = computed(() =>
  Array.from({ length: 60 }, (_, minute) => {
    return {
      value: String(minute).padStart(2, '0'),
      disabled:
        invalidRange.value ||
        !isMinuteWithinRange(baseTime.value.hour, minute, minValue.value, maxValue.value),
    }
  }),
)
const secondOptions = computed(() =>
  Array.from({ length: 60 }, (_, second) => {
    const candidate = new Time(baseTime.value.hour, baseTime.value.minute, second)
    return {
      value: String(second).padStart(2, '0'),
      disabled: invalidRange.value || !isTimeWithinRange(candidate, minValue.value, maxValue.value),
    }
  }),
)

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) isOpen.value = false
  },
)

function updateHour(rawValue: unknown) {
  if (props.disabled || props.readonly || invalidRange.value || typeof rawValue !== 'string') return
  const hour = Number(rawValue)
  if (!Number.isInteger(hour) || hour < 0 || hour > 23) return

  const candidate = clampTimeToRange(
    new Time(hour, baseTime.value.minute, baseTime.value.second),
    minValue.value,
    maxValue.value,
  )
  emit('update:modelValue', formatTimeValue(candidate))
}

function updateMinute(rawValue: unknown) {
  if (props.disabled || props.readonly || invalidRange.value || typeof rawValue !== 'string') return
  const minute = Number(rawValue)
  if (!Number.isInteger(minute) || minute < 0 || minute > 59) return

  const candidate = clampTimeToRange(
    new Time(baseTime.value.hour, minute, baseTime.value.second),
    minValue.value,
    maxValue.value,
  )
  emit('update:modelValue', formatTimeValue(candidate))
}

function updateSecond(rawValue: unknown) {
  if (props.disabled || props.readonly || invalidRange.value || typeof rawValue !== 'string') return
  const second = Number(rawValue)
  if (!Number.isInteger(second) || second < 0 || second > 59) return

  const candidate = new Time(baseTime.value.hour, baseTime.value.minute, second)
  if (isTimeWithinRange(candidate, minValue.value, maxValue.value)) {
    emit('update:modelValue', formatTimeValue(candidate))
  }
}
function clearTime() {
  if (!props.disabled && !props.readonly) emit('update:modelValue', '')
}

function formatHour(hour: number): string {
  return String(hour).padStart(2, '0')
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
          :class="clearable && modelValue && !readonly ? 'pe-9' : undefined"
          :disabled="disabled"
          :aria-label="ariaLabel"
          :aria-labelledby="ariaLabelledby"
          :aria-describedby="describedBy"
          :aria-invalid="isInvalid ? 'true' : undefined"
          :aria-readonly="readonly ? 'true' : undefined"
        >
          <Clock3 class="size-4 shrink-0" aria-hidden="true" />
          <span
            class="min-w-0 flex-1 truncate"
            :class="modelValue ? undefined : 'text-muted-foreground'"
          >
            {{ displayValue }}
          </span>
        </Button>
      </PopoverTrigger>

      <Button
        v-if="clearable && modelValue && !readonly"
        type="button"
        variant="ghost"
        size="icon-xs"
        class="absolute end-1 top-1/2 -translate-y-1/2"
        :disabled="disabled"
        aria-label="清除时间"
        @click="clearTime"
      >
        <X class="size-4" />
      </Button>
    </div>

    <Input v-if="name" type="hidden" :name="name" :model-value="modelValue" :disabled="disabled" />
    <PopoverContent
      align="start"
      aria-label="选择时间"
      class="w-80 max-w-(--reka-popover-content-available-width) max-h-(--reka-popover-content-available-height) overflow-y-auto p-3"
    >
      <TimeOptions
        :hour="selectedHour"
        :second="selectedSecond"
        :minute="selectedMinute"
        :hours="hourOptions"
        :minutes="minuteOptions"
        :seconds="secondOptions"
        :disabled="disabled || readonly || invalidRange"
        class="w-full"
        @select:hour="updateHour"
        @select:minute="updateMinute"
        @select:second="updateSecond"
      />
    </PopoverContent>
  </Popover>
</template>
