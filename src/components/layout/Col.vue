<!-- eslint-disable vue/multi-word-component-names -->
<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed } from 'vue'

import { cn } from '@/lib/utils'

import './layout.css'
import type { LayoutTag } from './layout'
import { resolveLayoutTag } from './layout'

export interface ColumnSettings {
  span?: number
  offset?: number
  push?: number
  pull?: number
}

export type ResponsiveColumn = number | ColumnSettings

interface ResolvedColumn {
  span: number
  offset: number
  push: number
  pull: number
}

const props = withDefaults(
  defineProps<{
    tag?: LayoutTag
    span?: number
    offset?: number
    push?: number
    pull?: number
    sm?: ResponsiveColumn
    md?: ResponsiveColumn
    lg?: ResponsiveColumn
    xl?: ResponsiveColumn
    xxl?: ResponsiveColumn
    class?: HTMLAttributes['class']
  }>(),
  {
    tag: 'div',
    span: 24,
    offset: 0,
    push: 0,
    pull: 0,
  },
)

const breakpoints = ['sm', 'md', 'lg', 'xl', 'xxl'] as const

function normalize(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.min(24, Math.max(0, Math.trunc(value)))
    : fallback
}

function applyResponsive(current: ResolvedColumn, value?: ResponsiveColumn): ResolvedColumn {
  if (typeof value === 'number') return { ...current, span: normalize(value, current.span) }
  if (!value) return current

  return {
    span: normalize(value.span, current.span),
    offset: normalize(value.offset, current.offset),
    push: normalize(value.push, current.push),
    pull: normalize(value.pull, current.pull),
  }
}

const sizes = computed(() => {
  const base: ResolvedColumn = {
    span: normalize(props.span, 24),
    offset: normalize(props.offset, 0),
    push: normalize(props.push, 0),
    pull: normalize(props.pull, 0),
  }
  const sm = applyResponsive(base, props.sm)
  const md = applyResponsive(sm, props.md)
  const lg = applyResponsive(md, props.lg)
  const xl = applyResponsive(lg, props.xl)
  const xxl = applyResponsive(xl, props.xxl)

  return { base, sm, md, lg, xl, xxl }
})
const style = computed(() => {
  const result: Record<string, number> = {}

  for (const breakpoint of ['base', ...breakpoints] as const) {
    const suffix = breakpoint === 'base' ? '' : `-${breakpoint}`
    const size = sizes.value[breakpoint]

    result[`--col-span${suffix}`] = size.span
    result[`--col-offset${suffix}`] = size.offset
    result[`--col-position${suffix}`] = size.push - size.pull
  }

  return result
})
const tag = computed(() => resolveLayoutTag(props.tag))
</script>

<template>
  <component
    :is="tag"
    :class="cn('phoenix-col', props.class)"
    :style="style"
    data-layout-col
    :data-col-span="sizes.base.span"
    :data-col-offset="sizes.base.offset"
    :data-col-push="sizes.base.push"
    :data-col-pull="sizes.base.pull"
    :data-col-span-sm="sizes.sm.span"
    :data-col-offset-sm="sizes.sm.offset"
    :data-col-push-sm="sizes.sm.push"
    :data-col-pull-sm="sizes.sm.pull"
    :data-col-span-md="sizes.md.span"
    :data-col-offset-md="sizes.md.offset"
    :data-col-push-md="sizes.md.push"
    :data-col-pull-md="sizes.md.pull"
    :data-col-span-lg="sizes.lg.span"
    :data-col-offset-lg="sizes.lg.offset"
    :data-col-push-lg="sizes.lg.push"
    :data-col-pull-lg="sizes.lg.pull"
    :data-col-span-xl="sizes.xl.span"
    :data-col-offset-xl="sizes.xl.offset"
    :data-col-push-xl="sizes.xl.push"
    :data-col-pull-xl="sizes.xl.pull"
    :data-col-span-xxl="sizes.xxl.span"
    :data-col-offset-xxl="sizes.xxl.offset"
    :data-col-push-xxl="sizes.xxl.push"
    :data-col-pull-xxl="sizes.xxl.pull"
  >
    <slot />
  </component>
</template>
