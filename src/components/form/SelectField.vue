<script setup lang="ts">
import type { FormControlProps, SelectOption } from './types'
import { computed, ref } from 'vue'
import { XIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { cn } from '@/lib/utils'

defineOptions({ inheritAttrs: false })
interface Props extends FormControlProps {
  modelValue?: string
  options: readonly SelectOption[]
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  disabled: false,
  readonly: false,
  invalid: false,
  clearable: true,
  required: false,
})
const rootElement = ref<HTMLDivElement | null>(null)

function handleCloseAutoFocus(event: Event) {
  const activeElement = document.activeElement
  const trigger = rootElement.value?.querySelector('[data-slot="select-trigger"]')
  const closingContent = event.target

  if (
    activeElement instanceof HTMLElement &&
    activeElement !== document.body &&
    activeElement !== document.documentElement &&
    activeElement !== trigger &&
    !(closingContent instanceof HTMLElement && closingContent.contains(activeElement))
  ) {
    event.preventDefault()
  }
}
const selectedLabel = computed(
  () =>
    props.options.find((option) => option.value === props.modelValue)?.label ||
    props.modelValue ||
    props.placeholder ||
    '',
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function updateModelValue(value: unknown) {
  emit('update:modelValue', typeof value === 'string' ? value : '')
}

function clear() {
  emit('update:modelValue', '')
  rootElement.value?.querySelector<HTMLElement>('[data-slot="select-trigger"]')?.focus()
}
</script>

<template>
  <div ref="rootElement" class="relative w-full min-w-0">
    <Select
      :model-value="props.modelValue"
      :disabled="props.disabled"
      :required="props.required"
      :open="props.readonly ? false : undefined"
      @update:model-value="updateModelValue"
    >
      <SelectTrigger
        v-bind="$attrs"
        :id="props.id"
        :aria-invalid="props.invalid ? 'true' : undefined"
        :aria-describedby="props.describedBy || undefined"
        :aria-readonly="props.readonly ? 'true' : undefined"
        :class="
          cn(
            'w-full min-w-0',
            props.clearable &&
              props.modelValue &&
              !props.disabled &&
              !props.readonly &&
              'pr-9 [&>svg]:hidden',
            props.class,
          )
        "
      >
        <SelectValue :placeholder="props.placeholder">{{ selectedLabel }}</SelectValue>
      </SelectTrigger>
      <SelectContent position="popper" @close-auto-focus="handleCloseAutoFocus">
        <SelectItem
          v-for="option in props.options"
          :key="option.value"
          :value="option.value"
          :disabled="option.disabled"
        >
          {{ option.label }}
        </SelectItem>
      </SelectContent>
    </Select>
    <Button
      v-if="props.clearable && props.modelValue && !props.disabled && !props.readonly"
      type="button"
      variant="ghost"
      size="icon-xs"
      class="absolute top-1/2 right-1 -translate-y-1/2"
      aria-label="清空选择"
      @click="clear"
    >
      <XIcon class="size-4" aria-hidden="true" />
    </Button>
  </div>
</template>
