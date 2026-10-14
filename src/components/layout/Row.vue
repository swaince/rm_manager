<!-- eslint-disable vue/multi-word-component-names -->
<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed } from 'vue'

import { cn } from '@/lib/utils'

import './layout.css'
import type { LayoutTag, RowAlign, RowJustify } from './layout'
import { resolveLayoutTag } from './layout'

const props = withDefaults(
  defineProps<{
    tag?: LayoutTag
    gutter?: number | [number, number]
    justify?: RowJustify
    align?: RowAlign
    class?: HTMLAttributes['class']
  }>(),
  {
    tag: 'div',
    gutter: 0,
    justify: 'start',
    align: 'stretch',
  },
)

const tag = computed(() => resolveLayoutTag(props.tag))
const gutter = computed(() => {
  const [horizontal, vertical] = Array.isArray(props.gutter) ? props.gutter : [props.gutter, 0]
  const normalize = (value: number) => (Number.isFinite(value) ? Math.max(0, value) : 0)

  return [normalize(horizontal), normalize(vertical)] as const
})
const style = computed(() => ({
  '--layout-gutter-x': `${gutter.value[0]}px`,
  '--layout-gutter-y': `${gutter.value[1]}px`,
  justifyContent:
    props.justify === 'start' ? 'flex-start' : props.justify === 'end' ? 'flex-end' : props.justify,
  alignItems:
    props.align === 'start' ? 'flex-start' : props.align === 'end' ? 'flex-end' : props.align,
}))
</script>

<template>
  <component
    :is="tag"
    :class="cn('phoenix-row', props.class)"
    :style="style"
    data-layout-row
    :data-gutter-x="gutter[0]"
    :data-gutter-y="gutter[1]"
    :data-justify="props.justify"
    :data-align="props.align"
  >
    <slot />
  </component>
</template>
