<script setup lang="ts">
import { computed } from 'vue'
import { ScrollArea } from '@/components/ui/scroll-area'

defineOptions({ inheritAttrs: false })

const props = defineProps<{
  title?: string
  description?: string
  maxHeight?: number | string
}>()

const emit = defineEmits<{
  submit: [event: SubmitEvent]
}>()

const isBounded = computed(() => props.maxHeight !== undefined)
const boundedStyle = computed(() => {
  if (!isBounded.value) return undefined

  return {
    maxHeight: typeof props.maxHeight === 'number' ? `${props.maxHeight}px` : props.maxHeight,
  }
})

function handleSubmit(event: SubmitEvent) {
  event.preventDefault()
  emit('submit', event)
}
</script>

<template>
  <form
    v-bind="$attrs"
    novalidate
    :style="boundedStyle"
    :class="
      isBounded
        ? 'flex min-h-0 w-full flex-col overflow-hidden rounded-xl border border-border bg-card text-card-foreground'
        : 'flex w-full flex-col rounded-xl border border-border bg-card text-card-foreground'
    "
    @submit="handleSubmit"
  >
    <header
      v-if="$slots.header || title || description"
      class="form-header shrink-0 border-b border-border px-5 py-4"
    >
      <slot name="header">
        <div v-if="title" class="text-base font-semibold text-foreground">
          {{ title }}
        </div>
        <p v-if="description" class="mt-1 text-sm text-muted-foreground">
          {{ description }}
        </p>
      </slot>
    </header>
    <ScrollArea
      v-if="isBounded"
      data-form-layout-body
      class="grid min-h-0 flex-1 [&>[data-slot=scroll-area-viewport]]:min-h-0 [&>[data-slot=scroll-area-viewport]]:h-auto"
    >
      <div class="px-5 py-4">
        <slot />
      </div>
    </ScrollArea>
    <div v-else data-form-layout-body class="px-5 py-4">
      <slot />
    </div>
    <footer v-if="$slots.footer" class="shrink-0 border-t border-border bg-form-footer px-5 py-3">
      <slot name="footer" />
    </footer>
  </form>
</template>
