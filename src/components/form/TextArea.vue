<script setup lang="ts">
import type { FormControlProps } from './types'
import { XIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'

defineOptions({ inheritAttrs: false })
interface Props extends FormControlProps {
  modelValue?: string
  rows?: number
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  rows: 3,
  disabled: false,
  readonly: false,
  invalid: false,
  clearable: false,
  required: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function updateModelValue(value: string | number) {
  emit('update:modelValue', String(value))
}

function clear() {
  emit('update:modelValue', '')
}
</script>

<template>
  <div class="relative w-full min-w-0">
    <Textarea
      v-bind="$attrs"
      :id="props.id"
      :model-value="props.modelValue"
      :rows="props.rows"
      :disabled="props.disabled"
      :readonly="props.readonly"
      :required="props.required"
      :placeholder="props.placeholder"
      :aria-invalid="props.invalid ? 'true' : undefined"
      :aria-describedby="props.describedBy || undefined"
      :class="cn(props.clearable && props.modelValue && 'pr-9', props.class)"
      @update:model-value="updateModelValue"
    />
    <Button
      v-if="props.clearable && props.modelValue && !props.disabled && !props.readonly"
      type="button"
      variant="ghost"
      size="icon-xs"
      class="absolute top-2 right-1"
      aria-label="清空文本"
      @click="clear"
    >
      <XIcon class="size-4" aria-hidden="true" />
    </Button>
  </div>
</template>
