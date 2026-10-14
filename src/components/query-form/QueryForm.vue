<script setup lang="ts">
import type { HTMLAttributes, PropType, VNode } from 'vue'
import {
  Comment,
  Fragment,
  cloneVNode,
  computed,
  defineComponent,
  h,
  isVNode,
  onBeforeUnmount,
  onMounted,
  ref,
  useId,
  useSlots,
  watch,
} from 'vue'
import { FormLayout } from '@/components/form'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { cn } from '@/lib/utils'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    loading?: boolean
    disabled?: boolean
    moreCount?: number
    title?: string
    description?: string
    class?: HTMLAttributes['class']
  }>(),
  {
    loading: false,
    disabled: false,
    moreCount: 0,
  },
)

const emit = defineEmits<{
  submit: []
  reset: []
  'advanced-open': []
  'advanced-cancel': []
  'advanced-confirm': [event: SubmitEvent]
  'advanced-reset': []
  'visible-count-change': [payload: { visibleCount: number; totalCount: number }]
}>()

const slots = useSlots()
const descriptionId = useId()
const viewportWidth = ref(0)
const advancedOpen = ref(false)
const lastVisiblePayload = ref('')
const actionsDisabled = computed(() => props.loading || props.disabled)
const fields = computed(() => flattenSlotNodes(slots.default?.() ?? []))
const hasAdvanced = computed(() => Boolean(slots.advanced))
const fieldLimit = computed(() => {
  if (!hasAdvanced.value) return fields.value.length
  const capacity =
    viewportWidth.value < 768
      ? 1
      : viewportWidth.value < 1024
        ? 3
        : viewportWidth.value < 1280
          ? 5
          : 7
  return Math.min(fields.value.length, capacity)
})
const hasHiddenFields = computed(() => hasAdvanced.value && fields.value.length > fieldLimit.value)
const columns = computed(() => {
  if (viewportWidth.value < 768) return 1
  if (viewportWidth.value < 1024) return 2
  if (viewportWidth.value < 1280) return 3

  const count = fieldLimit.value
  return count > 0 && count % 4 === 0 && count % 3 !== 0 ? 3 : 4
})
const actionsEmbedded = computed(() => fields.value.length > 0 && columns.value === 1)
const moveLastField = computed(
  () => fieldLimit.value > 0 && columns.value > 1 && fieldLimit.value % columns.value === 0,
)

const QueryFieldItems = defineComponent({
  name: 'QueryFieldItems',
  props: {
    nodes: { type: Array as PropType<VNode[]>, required: true },
    limit: { type: Number, required: true },
    embedActions: { type: Boolean, default: false },
    moveLast: { type: Boolean, default: false },
  },
  setup(fieldProps, { slots: fieldSlots }) {
    return () => {
      const shouldEmbedActions =
        fieldProps.embedActions && fieldProps.limit > 0 && fieldProps.nodes.length > 0
      const items = fieldProps.nodes.map((node, index) => {
        const visible = index < fieldProps.limit
        const children = [cloneVNode(node)]

        if (visible && shouldEmbedActions && index === fieldProps.limit - 1) {
          children.push(...(fieldSlots.actions?.({ embedded: true }) ?? []))
        }

        return h(
          'div',
          {
            class: visible ? 'query-form-field min-w-0' : 'query-form-field hidden min-w-0',
            'data-slot': 'query-form-field',
            style:
              visible && fieldProps.moveLast && index === fieldProps.limit - 1
                ? { gridColumn: '1 / span 2' }
                : undefined,
            'aria-hidden': visible ? undefined : 'true',
            hidden: visible ? undefined : true,
            inert: visible ? undefined : '',
          },
          children,
        )
      })

      if (!shouldEmbedActions) {
        items.push(...(fieldSlots.actions?.({ embedded: false }) ?? []))
      }

      return h(Fragment, null, items)
    }
  },
})

const QueryActions = defineComponent({
  name: 'QueryActions',
  props: {
    hasHiddenFields: { type: Boolean, required: true },
    moreCount: { type: Number, required: true },
    loading: { type: Boolean, required: true },
    disabled: { type: Boolean, required: true },
    embedded: { type: Boolean, default: false },
    openAdvanced: { type: Function as PropType<() => void>, required: true },
  },
  setup(actionProps, { slots: actionSlots }) {
    return () =>
      h(
        'div',
        {
          class: actionProps.embedded
            ? 'mt-3 flex min-w-0 flex-wrap items-center gap-2'
            : 'flex min-w-0 flex-wrap items-center gap-2',
          'data-slot': 'query-form-actions',
        },
        [
          ...(actionSlots.default?.() ?? []),
          actionProps.hasHiddenFields
            ? h(
                Button,
                {
                  type: 'button',
                  size: 'default',
                  variant: 'link',
                  class: 'px-0 text-muted-foreground hover:text-foreground',
                  'aria-label': '更多条件',
                  disabled: actionProps.loading || actionProps.disabled,
                  onClick: actionProps.openAdvanced,
                },
                () => [
                  '更多条件',
                  actionProps.moreCount > 0
                    ? h(
                        'span',
                        { class: 'ml-1 tabular-nums', 'aria-hidden': 'true' },
                        String(actionProps.moreCount),
                      )
                    : null,
                ],
              )
            : null,
        ],
      )
  },
})

function flattenSlotNodes(nodes: VNode[]): VNode[] {
  const flattened: VNode[] = []

  for (const node of nodes) {
    if (!isVNode(node) || node.type === Comment) continue

    if (node.type === Fragment && Array.isArray(node.children)) {
      flattened.push(...flattenSlotNodes(node.children as VNode[]))
    } else {
      flattened.push(node)
    }
  }

  return flattened
}

function emitVisibleCount() {
  const payload = {
    visibleCount: fieldLimit.value,
    totalCount: fields.value.length,
  }
  const serialized = JSON.stringify(payload)
  if (serialized === lastVisiblePayload.value) return

  lastVisiblePayload.value = serialized
  emit('visible-count-change', payload)
}

function submit() {
  if (actionsDisabled.value) return
  emit('submit')
}

function reset() {
  if (actionsDisabled.value) return
  emit('reset')
}

function openAdvanced() {
  if (actionsDisabled.value || !hasHiddenFields.value) return
  advancedOpen.value = true
  emit('advanced-open')
}

function cancelAdvanced() {
  if (!advancedOpen.value) return
  advancedOpen.value = false
  emit('advanced-cancel')
}

function handleDialogOpenChange(open: boolean) {
  if (open) {
    openAdvanced()
  } else {
    cancelAdvanced()
  }
}

function resetAdvanced() {
  if (actionsDisabled.value) return
  emit('advanced-reset')
}

function confirmAdvanced(event: SubmitEvent) {
  if (actionsDisabled.value) return

  const form = event.currentTarget
  if (form instanceof HTMLFormElement && !form.reportValidity()) return

  emit('advanced-confirm', event)
  advancedOpen.value = false
}

function setViewportWidth() {
  viewportWidth.value = window.innerWidth
}

watch(() => [fieldLimit.value, fields.value.length] as const, emitVisibleCount)

onMounted(() => {
  setViewportWidth()
  window.addEventListener('resize', setViewportWidth)
  emitVisibleCount()
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', setViewportWidth)
})
</script>

<template>
  <form
    v-bind="$attrs"
    :aria-busy="props.loading || undefined"
    :class="cn('w-full min-w-0', props.class)"
    novalidate
    @submit.prevent="submit"
    @reset.prevent="reset"
  >
    <div v-if="props.title || props.description" class="mb-4 min-w-0">
      <h3 v-if="props.title" class="text-sm font-semibold">{{ props.title }}</h3>
      <p v-if="props.description" class="mt-1 text-sm text-muted-foreground">
        {{ props.description }}
      </p>
    </div>

    <div
      data-slot="query-form-fields"
      :data-visible-count="fieldLimit"
      :data-total-count="fields.length"
      :data-columns="columns"
      :style="{ '--query-columns': columns }"
      :class="cn('grid w-full min-w-0 grid-cols-1 gap-x-4 gap-y-3', 'query-form-fields')"
    >
      <QueryFieldItems
        :nodes="fields"
        :limit="fieldLimit"
        :embed-actions="actionsEmbedded"
        :move-last="moveLastField"
      >
        <template #actions="{ embedded }">
          <QueryActions
            :has-hidden-fields="hasHiddenFields"
            :more-count="props.moreCount"
            :loading="props.loading"
            :disabled="props.disabled"
            :embedded="embedded"
            :open-advanced="openAdvanced"
          >
            <slot
              name="actions"
              :loading="props.loading"
              :disabled="actionsDisabled"
              :submit="submit"
              :reset="reset"
              :open-advanced="openAdvanced"
              :has-advanced="hasHiddenFields"
              :more-count="props.moreCount"
            >
              <Button type="submit" :disabled="actionsDisabled">
                {{ props.loading ? '查询中…' : '查询' }}
              </Button>
              <Button type="button" variant="outline" :disabled="actionsDisabled" @click="reset">
                重置
              </Button>
            </slot>
          </QueryActions>
        </template>
      </QueryFieldItems>
    </div>
  </form>

  <Dialog :open="advancedOpen" @update:open="handleDialogOpenChange">
    <DialogContent
      class="max-h-[calc(100dvh-2rem)] overflow-hidden p-0 sm:max-w-4xl"
      :aria-describedby="descriptionId"
    >
      <FormLayout
        title="更多查询条件"
        description="设置完整筛选条件后确认查询；取消将丢弃本次修改。"
        max-height="calc(100dvh - 2rem)"
        class="min-h-0 flex-1 rounded-none border-0 bg-transparent"
        @submit="confirmAdvanced"
        @reset.prevent="resetAdvanced"
      >
        <template #header>
          <DialogHeader>
            <DialogTitle>更多查询条件</DialogTitle>
            <DialogDescription :id="descriptionId">
              设置完整筛选条件后确认查询；取消将丢弃本次修改。
            </DialogDescription>
          </DialogHeader>
        </template>

        <div
          data-slot="query-form-advanced-fields"
          class="grid min-w-0 grid-cols-1 gap-x-5 gap-y-5 md:grid-cols-2"
        >
          <slot name="advanced" />
        </div>

        <template #footer>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <Button
              type="button"
              variant="outline"
              :disabled="actionsDisabled"
              @click="resetAdvanced"
            >
              重置
            </Button>
            <div class="flex flex-wrap justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                :disabled="actionsDisabled"
                @click="cancelAdvanced"
              >
                取消
              </Button>
              <Button type="submit" :disabled="actionsDisabled">
                {{ props.loading ? '查询中…' : '确认' }}
              </Button>
            </div>
          </div>
        </template>
      </FormLayout>
    </DialogContent>
  </Dialog>
</template>

<style>
@media (min-width: 48rem) {
  .query-form-fields {
    grid-template-columns: repeat(var(--query-columns), max-content minmax(0, 1fr));
  }

  .query-form-field:not([hidden]) {
    display: grid;
    grid-column: span 2;
    grid-template-columns: subgrid;
  }

  .query-form-fields > [data-slot='query-form-actions'] {
    grid-column: span 2;
  }

  .query-form-field [data-slot='field'] {
    display: grid;
    grid-column: span 2;
    grid-template-columns: subgrid;
    align-items: center;
    column-gap: inherit;
  }

  .query-form-field [data-slot='field-label'] {
    width: max-content;
    justify-self: end;
    justify-content: flex-end;
    text-align: right;
  }

  .query-form-field [data-slot='field-description'],
  .query-form-field [data-slot='field-error'] {
    grid-column: 2;
    min-width: 0;
  }
}
</style>
