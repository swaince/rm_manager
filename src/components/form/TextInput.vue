<script setup lang="ts">
import type { FormControlProps } from './types'
import { XIcon } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

defineOptions({ inheritAttrs: false })
interface Props extends FormControlProps {
  modelValue?: string
  type?: 'text' | 'password' | 'number' | 'email' | 'tel' | 'url' | 'search'
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  type: 'text',
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
    <Input
      v-bind="$attrs"
      :id="props.id"
      :model-value="props.modelValue"
      :type="props.type"
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
      class="absolute top-1/2 right-1 -translate-y-1/2"
      aria-label="清空输入"
      @click="clear"
    >
      <XIcon class="size-4" aria-hidden="true" />
    </Button>
  </div>
</template>
