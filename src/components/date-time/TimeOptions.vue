<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'

interface TimeOption {
  value: string
  disabled: boolean
}

const props = withDefaults(
  defineProps<{
    hour: string
    minute: string
    second: string
    hours: TimeOption[]
    minutes: TimeOption[]
    seconds: TimeOption[]
    disabled?: boolean
  }>(),
  { disabled: false },
)

const emit = defineEmits<{
  'select:hour': [value: string]
  'select:minute': [value: string]
  'select:second': [value: string]
}>()

const hourList = ref<HTMLDivElement>()
const minuteList = ref<HTMLDivElement>()
const secondList = ref<HTMLDivElement>()
watch(
  [() => props.hour, () => props.minute, () => props.second, hourList, minuteList, secondList],
  async () => {
    await nextTick()
    for (const list of [hourList.value, minuteList.value, secondList.value]) {
      const selected = list?.querySelector<HTMLElement>('[aria-selected="true"]')
      const viewport = list?.closest<HTMLElement>('[data-slot="scroll-area-viewport"]')
      if (!selected || !viewport) continue
      viewport.scrollTop +=
        selected.getBoundingClientRect().top -
        viewport.getBoundingClientRect().top -
        (viewport.clientHeight - selected.offsetHeight) / 2
    }
  },
  { flush: 'post' },
)

function tabStop(options: TimeOption[], value: string): string | undefined {
  return (
    options.find((option) => option.value === value && !option.disabled)?.value ??
    options.find((option) => !option.disabled)?.value
  )
}

const hourTabStop = computed(() => tabStop(props.hours, props.hour))
const minuteTabStop = computed(() => tabStop(props.minutes, props.minute))
const secondTabStop = computed(() => tabStop(props.seconds, props.second))

function handleKeydown(event: KeyboardEvent, field: 'hour' | 'minute' | 'second') {
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return

  const list = { hour: hourList.value, minute: minuteList.value, second: secondList.value }[field]
  if (!list) return

  const enabled = Array.from(list.querySelectorAll<HTMLButtonElement>('[role="option"]')).filter(
    (option) => !option.disabled,
  )
  if (enabled.length === 0) return

  const currentIndex = enabled.indexOf(document.activeElement as HTMLButtonElement)
  const nextIndex =
    event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? enabled.length - 1
        : Math.max(
            0,
            Math.min(enabled.length - 1, currentIndex + (event.key === 'ArrowDown' ? 1 : -1)),
          )

  event.preventDefault()
  enabled[nextIndex]?.focus()
}
</script>

<template>
  <div class="flex min-w-0 items-center gap-2">
    <div class="min-w-0 flex-1">
      <ScrollArea class="h-40 sm:h-60 rounded-md border">
        <div
          ref="hourList"
          role="listbox"
          aria-label="小时"
          aria-orientation="vertical"
          class="grid gap-1 py-1 ps-1 pe-3"
          @keydown="handleKeydown($event, 'hour')"
        >
          <Button
            v-for="option in hours"
            :key="option.value"
            type="button"
            role="option"
            size="sm"
            :variant="option.value === hour ? 'secondary' : 'ghost'"
            :aria-label="`选择小时 ${option.value}`"
            :aria-selected="option.value === hour"
            :tabindex="option.value === hourTabStop ? 0 : -1"
            :disabled="disabled || option.disabled"
            class="w-full tabular-nums"
            @click="emit('select:hour', option.value)"
          >
            {{ option.value }}
          </Button>
        </div>
      </ScrollArea>
    </div>
    <span aria-hidden="true" class="shrink-0 text-xl font-medium">:</span>
    <div class="min-w-0 flex-1">
      <ScrollArea class="h-40 sm:h-60 rounded-md border">
        <div
          ref="minuteList"
          role="listbox"
          aria-label="分钟"
          aria-orientation="vertical"
          class="grid gap-1 py-1 ps-1 pe-3"
          @keydown="handleKeydown($event, 'minute')"
        >
          <Button
            v-for="option in minutes"
            :key="option.value"
            type="button"
            role="option"
            size="sm"
            :variant="option.value === minute ? 'secondary' : 'ghost'"
            :aria-label="`选择分钟 ${option.value}`"
            :aria-selected="option.value === minute"
            :tabindex="option.value === minuteTabStop ? 0 : -1"
            :disabled="disabled || option.disabled"
            class="w-full tabular-nums"
            @click="emit('select:minute', option.value)"
          >
            {{ option.value }}
          </Button>
        </div>
      </ScrollArea>
    </div>
    <span aria-hidden="true" class="shrink-0 text-xl font-medium">:</span>
    <div class="min-w-0 flex-1">
      <ScrollArea class="h-40 sm:h-60 rounded-md border">
        <div
          ref="secondList"
          role="listbox"
          aria-label="秒"
          aria-orientation="vertical"
          class="grid gap-1 py-1 ps-1 pe-3"
          @keydown="handleKeydown($event, 'second')"
        >
          <Button
            v-for="option in seconds"
            :key="option.value"
            type="button"
            role="option"
            size="sm"
            :variant="option.value === second ? 'secondary' : 'ghost'"
            :aria-label="`选择秒 ${option.value}`"
            :aria-selected="option.value === second"
            :tabindex="option.value === secondTabStop ? 0 : -1"
            :disabled="disabled || option.disabled"
            class="w-full tabular-nums"
            @click="emit('select:second', option.value)"
          >
            {{ option.value }}
          </Button>
        </div>
      </ScrollArea>
    </div>
  </div>
</template>
