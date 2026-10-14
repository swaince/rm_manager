<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { HTMLAttributes } from 'vue'
import { Button, type ButtonVariants } from '@/components/ui/button'
import { copyText } from '@/lib/clipboard'

defineOptions({ inheritAttrs: false })

interface Props {
  text: string
  disabled?: boolean
  label?: string
  variant?: ButtonVariants['variant']
  size?: ButtonVariants['size']
  class?: HTMLAttributes['class']
}

const props = withDefaults(defineProps<Props>(), {
  label: '复制',
})

const emit = defineEmits<{
  'copy-success': [text: string]
  'copy-error': [error: unknown]
}>()

const isPending = ref(false)
const outcome = ref<'idle' | 'success' | 'error'>('idle')
const errorMessage = ref('')

const buttonLabel = computed(() => {
  if (isPending.value) return '复制中…'
  if (outcome.value === 'success') return '已复制'
  if (outcome.value === 'error') return '复制失败'
  return props.label
})

watch(
  () => props.text,
  () => {
    outcome.value = 'idle'
    errorMessage.value = ''
  },
)

async function copy() {
  if (props.disabled || isPending.value) return

  const copiedText = props.text
  isPending.value = true
  outcome.value = 'idle'
  errorMessage.value = ''

  try {
    try {
      await copyText(copiedText)
    } catch (error) {
      if (props.text === copiedText) {
        outcome.value = 'error'
        errorMessage.value = '复制失败，请手动选择并复制文本。'
      }
      emit('copy-error', error)
      return
    }

    if (props.text === copiedText) outcome.value = 'success'
    emit('copy-success', copiedText)
  } finally {
    isPending.value = false
  }
}
</script>

<template>
  <div class="inline-flex max-w-full flex-col items-start">
    <Button
      v-bind="$attrs"
      type="button"
      :disabled="props.disabled"
      :variant="props.variant"
      :size="props.size"
      :class="props.class"
      :aria-busy="isPending"
      :aria-disabled="props.disabled || isPending"
      @click="copy"
    >
      <span aria-live="polite" aria-atomic="true">{{ buttonLabel }}</span>
    </Button>
    <span v-if="errorMessage" role="alert" class="text-sm text-destructive">
      {{ errorMessage }}
    </span>
  </div>
</template>
