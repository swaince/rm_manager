<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { getLocalTimeZone, Time, today } from '@internationalized/date'
import { X } from '@lucide/vue'
import type { DateValue, TimeValue } from 'reka-ui'
import { Button } from '@/components/ui/button'
import DateCalendar from './DateCalendar.vue'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import TimeOptions from './TimeOptions.vue'
import {
  clampTimeToRange,
  formatTimeValue,
  isMinuteWithinRange,
  isTimeWithinRange,
  parseCalendarDate,
  parseTimeValue,
  timeToSeconds,
} from './date-time'
import type { DateTimeControlProps } from './types'

type DateTimePickerProps = Omit<DateTimeControlProps, 'name'> & { modelValue: string }
type ParsedDateTime = { date: DateValue; time: TimeValue }

const props = withDefaults(defineProps<DateTimePickerProps>(), {
  modelValue: '',
  disabled: false,
  readonly: false,
  invalid: false,
  describedBy: undefined,
  ariaLabel: undefined,
  ariaLabelledby: undefined,
  placeholder: '选择日期和时间',
  clearable: false,
  min: undefined,
  max: undefined,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const controlRoot = ref<HTMLElement>()
const isOpen = ref(false)
const activeTab = ref<'date' | 'time'>('date')
const draftDate = ref<DateValue>()
const draftTime = ref('')

const selectedValue = computed(() => parseDateTime(props.modelValue))
const minimum = computed(() => parseDateTime(props.min))
const maximum = computed(() => parseDateTime(props.max))
const invalidRange = computed(() => {
  if ((props.min && !minimum.value) || (props.max && !maximum.value)) return true
  return Boolean(
    minimum.value && maximum.value && compareDateTimes(minimum.value, maximum.value) > 0,
  )
})
const isInvalid = computed(() => {
  if (props.invalid || invalidRange.value) return true
  return Boolean(props.modelValue && (!selectedValue.value || !isWithinRange(selectedValue.value)))
})
const calendarMin = computed(() => (invalidRange.value ? undefined : minimum.value?.date))
const calendarMax = computed(() => (invalidRange.value ? undefined : maximum.value?.date))
const calendarPlaceholder = computed(
  () => draftDate.value ?? calendarMin.value ?? today(getLocalTimeZone()),
)
const timeMin = computed(() =>
  draftDate.value && minimum.value?.date.compare(draftDate.value) === 0
    ? minimum.value.time
    : undefined,
)
const timeMax = computed(() =>
  draftDate.value && maximum.value?.date.compare(draftDate.value) === 0
    ? maximum.value.time
    : undefined,
)
const selectedTime = computed(() => parseTimeValue(draftTime.value))
const baseTime = computed(() => selectedTime.value ?? timeMin.value ?? new Time(0, 0, 0))
const selectedHour = computed(() =>
  selectedTime.value ? String(selectedTime.value.hour).padStart(2, '0') : '',
)
const selectedMinute = computed(() =>
  selectedTime.value ? String(selectedTime.value.minute).padStart(2, '0') : '',
)
const selectedSecond = computed(() =>
  selectedTime.value ? String(selectedTime.value.second).padStart(2, '0') : '',
)
const hourOptions = computed(() =>
  Array.from({ length: 24 }, (_, hour) => {
    const hourStart = hour * 3600
    const hourEnd = hourStart + 3599
    const minSeconds = timeMin.value ? timeToSeconds(timeMin.value) : 0
    const maxSeconds = timeMax.value ? timeToSeconds(timeMax.value) : 86399
    return {
      value: String(hour).padStart(2, '0'),
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
        !isMinuteWithinRange(baseTime.value.hour, minute, timeMin.value, timeMax.value),
    }
  }),
)
const secondOptions = computed(() =>
  Array.from({ length: 60 }, (_, second) => {
    const candidate = new Time(baseTime.value.hour, baseTime.value.minute, second)
    return {
      value: String(second).padStart(2, '0'),
      disabled: invalidRange.value || !isTimeWithinRange(candidate, timeMin.value, timeMax.value),
    }
  }),
)
const canConfirm = computed(() => {
  const time = parseTimeValue(draftTime.value)
  return Boolean(
    isOpen.value &&
    !props.disabled &&
    !props.readonly &&
    !invalidRange.value &&
    draftDate.value &&
    time &&
    isWithinRange({ date: draftDate.value, time }),
  )
})

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) setOpen(false)
  },
)

function parseDateTime(value?: string): ParsedDateTime | undefined {
  if (!value) return undefined
  const match = /^(\d{4}-\d{2}-\d{2}) ((?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d)$/.exec(value)
  if (!match) return undefined

  const date = parseCalendarDate(match[1])
  const time = parseTimeValue(match[2])
  return date && time ? { date, time } : undefined
}

function compareDateTimes(left: ParsedDateTime, right: ParsedDateTime): number {
  return left.date.compare(right.date) || timeToSeconds(left.time) - timeToSeconds(right.time)
}

function isWithinRange(value: ParsedDateTime): boolean {
  return (
    (!minimum.value || compareDateTimes(value, minimum.value) >= 0) &&
    (!maximum.value || compareDateTimes(value, maximum.value) <= 0)
  )
}

function resetDraft() {
  draftDate.value = selectedValue.value?.date
  draftTime.value = selectedValue.value ? formatTimeValue(selectedValue.value.time) : ''
}

function setOpen(open: boolean) {
  resetDraft()
  if (open) activeTab.value = 'date'
  isOpen.value = open && !props.disabled
}

function updateDraftDate(value?: DateValue) {
  if (props.disabled || props.readonly || invalidRange.value) return
  draftDate.value = value
  if (value && !draftTime.value) {
    draftTime.value = formatTimeValue(timeMin.value ?? new Time(0, 0, 0))
  }
}

function isCalendarDateDisabled(date: DateValue): boolean {
  if (invalidRange.value) return true
  if (calendarMin.value && date.compare(calendarMin.value) < 0) return true
  if (calendarMax.value && date.compare(calendarMax.value) > 0) return true
  return false
}
function updateDraftHour(value: string) {
  if (props.disabled || props.readonly || !draftDate.value || invalidRange.value) return
  const hour = Number(value)
  if (!Number.isInteger(hour) || hour < 0 || hour > 23) return

  const candidate = clampTimeToRange(
    new Time(hour, baseTime.value.minute, baseTime.value.second),
    timeMin.value,
    timeMax.value,
  )
  draftTime.value = formatTimeValue(candidate)
}

function updateDraftMinute(value: string) {
  if (props.disabled || props.readonly || !draftDate.value || invalidRange.value) return
  const minute = Number(value)
  if (!Number.isInteger(minute) || minute < 0 || minute > 59) return

  const candidate = clampTimeToRange(
    new Time(baseTime.value.hour, minute, baseTime.value.second),
    timeMin.value,
    timeMax.value,
  )
  draftTime.value = formatTimeValue(candidate)
}

function updateDraftSecond(value: string) {
  if (props.disabled || props.readonly || !draftDate.value || invalidRange.value) return
  const second = Number(value)
  if (!Number.isInteger(second) || second < 0 || second > 59) return

  const candidate = new Time(baseTime.value.hour, baseTime.value.minute, second)
  if (isTimeWithinRange(candidate, timeMin.value, timeMax.value)) {
    draftTime.value = formatTimeValue(candidate)
  }
}
async function closePopover() {
  setOpen(false)
  await nextTick()
  controlRoot.value?.querySelector<HTMLButtonElement>('[data-date-time-picker-trigger]')?.focus()
}
async function confirm() {
  const time = parseTimeValue(draftTime.value)
  if (!canConfirm.value || !draftDate.value || !time) return

  emit('update:modelValue', `${draftDate.value.toString()} ${formatTimeValue(time)}`)
  await closePopover()
}

async function clearValue() {
  if (props.disabled || props.readonly) return
  emit('update:modelValue', '')
  await nextTick()
  controlRoot.value?.querySelector<HTMLButtonElement>('[data-date-time-picker-trigger]')?.focus()
}
</script>

<template>
  <div ref="controlRoot" class="relative w-full">
    <Popover :open="isOpen" @update:open="setOpen">
      <PopoverTrigger as-child>
        <Button
          :id="id"
          data-date-time-picker-trigger
          type="button"
          variant="outline"
          class="form-control-surface min-w-0 w-full justify-start border-input text-left font-normal"
          :class="clearable && modelValue && !readonly ? 'pe-9' : undefined"
          :disabled="disabled"
          :aria-invalid="isInvalid ? 'true' : undefined"
          :aria-describedby="describedBy"
          :aria-label="ariaLabel"
          :aria-labelledby="ariaLabelledby"
          :aria-readonly="readonly ? 'true' : undefined"
        >
          <span class="min-w-0 flex-1 truncate" :class="!modelValue ? 'text-muted-foreground' : ''">
            {{ modelValue || placeholder }}
          </span>
        </Button>
      </PopoverTrigger>

      <PopoverContent
        align="start"
        aria-label="选择日期和时间"
        class="flex max-h-(--reka-popover-content-available-height) w-auto max-w-(--reka-popover-content-available-width) flex-col overflow-hidden p-0"
      >
        <Tabs v-model="activeTab" class="flex h-96 min-h-0 w-80 max-w-full flex-col gap-0">
          <TabsList class="mx-3 mt-3 grid w-auto shrink-0 grid-cols-2">
            <TabsTrigger value="date">日期</TabsTrigger>
            <TabsTrigger value="time">时间</TabsTrigger>
          </TabsList>
          <TabsContent
            value="date"
            class="m-0 flex min-h-0 min-w-0 flex-1 items-center justify-center overflow-y-auto p-2"
          >
            <DateCalendar
              class="w-80 max-w-full [&_[data-slot=calendar-cell-trigger]]:size-10"
              :model-value="draftDate"
              :placeholder="calendarPlaceholder"
              :min-value="calendarMin"
              :max-value="calendarMax"
              :is-date-disabled="isCalendarDateDisabled"
              :disabled="disabled"
              :readonly="readonly"
              initial-focus
              :prevent-deselect="true"
              @update:model-value="updateDraftDate"
            />
          </TabsContent>
          <TabsContent
            value="time"
            class="m-0 flex min-h-0 min-w-0 flex-1 items-center justify-center overflow-y-auto p-3"
          >
            <TimeOptions
              class="w-full"
              :second="selectedSecond"
              :hour="selectedHour"
              :minute="selectedMinute"
              :hours="hourOptions"
              :seconds="secondOptions"
              :minutes="minuteOptions"
              :disabled="disabled || readonly || !draftDate || invalidRange"
              @select:hour="updateDraftHour"
              @select:minute="updateDraftMinute"
              @select:second="updateDraftSecond"
            />
          </TabsContent>
        </Tabs>
        <div class="flex shrink-0 justify-end gap-2 border-t p-3">
          <Button type="button" variant="outline" @click="closePopover">取消</Button>
          <Button
            type="button"
            :disabled="!canConfirm"
            aria-label="确认日期和时间"
            @click="confirm"
          >
            确定
          </Button>
        </div>
      </PopoverContent>
    </Popover>

    <Button
      v-if="clearable && modelValue && !readonly"
      type="button"
      variant="ghost"
      size="icon-xs"
      class="absolute end-1 top-1/2 -translate-y-1/2"
      :disabled="disabled"
      aria-label="清除日期和时间"
      @click="clearValue"
    >
      <X class="size-4" />
    </Button>
  </div>
</template>
