<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  Combobox,
  ComboboxAnchor,
  ComboboxEmpty,
  ComboboxGroup,
  ComboboxInput,
  ComboboxItem,
  ComboboxItemIndicator,
  ComboboxList,
} from '@/components/ui/combobox'
import { CheckIcon, XIcon } from '@lucide/vue'
import Button from '@/components/ui/button/Button.vue'
import { cn } from '@/lib/utils'
import type { SelectOption } from './types'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    modelValue?: string[]
    options: SelectOption[]
    id?: string
    disabled?: boolean
    readonly?: boolean
    invalid?: boolean
    describedBy?: string
    placeholder?: string
    clearable?: boolean
    required?: boolean
    class?: string
  }>(),
  {
    modelValue: () => [],
    disabled: false,
    readonly: false,
    invalid: false,
    clearable: false,
    required: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
  search: [query: string]
}>()

const query = ref('')
const filteredOptions = computed(() =>
  props.options
    .map((option, index) => ({ option, index }))
    .filter(({ option }) => option.label.toLowerCase().includes(query.value.trim().toLowerCase())),
)
const canClear = computed(
  () => props.clearable && props.modelValue.length > 0 && !props.disabled && !props.readonly,
)
const tokenOptions = computed(
  () => new Map(props.options.map((option, index) => [optionToken(index), option])),
)
const selectedTokens = computed(() => props.modelValue.map(tokenForValue))
const selectedOptions = computed(() =>
  props.modelValue.map((value) => ({
    value,
    label: props.options.find((option) => option.value === value)?.label ?? value,
  })),
)

watch(query, (value) => emit('search', value))

function optionToken(index: number) {
  return `form-option-${index}`
}

function tokenForValue(value: string) {
  const index = props.options.findIndex((option) => option.value === value)
  return index === -1 ? `missing:${encodeURIComponent(value)}` : optionToken(index)
}

function valueForToken(token: string) {
  if (token.startsWith('missing:')) return decodeURIComponent(token.slice('missing:'.length))

  return tokenOptions.value.get(token)?.value ?? ''
}

function updateSelection(tokens: unknown) {
  const values = Array.isArray(tokens) ? tokens.map((token) => valueForToken(String(token))) : []
  emit('update:modelValue', values)
}

function removeValue(value: string) {
  if (props.disabled || props.readonly) return

  emit(
    'update:modelValue',
    props.modelValue.filter((selected) => selected !== value),
  )
}

function clearValue() {
  if (!canClear.value) return

  emit('update:modelValue', [])
  query.value = ''
}
</script>

<template>
  <div :class="cn('w-full min-w-0', props.class)">
    <Combobox
      :model-value="selectedTokens"
      :open="readonly ? false : undefined"
      :disabled="disabled"
      :required="required"
      :multiple="true"
      :ignore-filter="true"
      :open-on-focus="true"
      @update:model-value="updateSelection"
    >
      <ComboboxAnchor
        class="form-control-surface"
        :aria-disabled="disabled || undefined"
        :data-disabled="disabled ? '' : undefined"
        :class="
          cn(
            'flex min-h-9 w-full min-w-0 flex-wrap items-center gap-1.5 rounded-md border border-input bg-background px-2 py-1 transition-colors focus-within:ring-3 focus-within:ring-ring/50 data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50',
            '[&_[data-slot=input-group]]:h-7 [&_[data-slot=input-group]]:rounded-none [&_[data-slot=input-group]]:border-0 [&_[data-slot=input-group]]:bg-transparent [&_[data-slot=input-group]]:shadow-none [&_[data-slot=input-group-addon]]:hidden',
            invalid && 'border-destructive',
          )
        "
      >
        <span
          v-for="selected in selectedOptions"
          :key="selected.value"
          class="inline-flex min-w-0 max-w-full items-center gap-0.5 rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground"
        >
          <span class="min-w-0 max-w-full truncate">{{ selected.label }}</span>
          <Button
            v-if="!readonly"
            type="button"
            variant="ghost"
            size="icon-xs"
            :disabled="disabled"
            :aria-label="`移除${selected.label}`"
            class="shrink-0"
            @click.stop="removeValue(selected.value)"
          >
            <XIcon class="size-3" aria-hidden="true" />
          </Button>
        </span>
        <div class="min-w-0 flex-1">
          <ComboboxInput
            v-bind="$attrs"
            :id="id"
            v-model="query"
            :placeholder="placeholder"
            :disabled="disabled"
            :readonly="readonly"
            :aria-readonly="readonly || undefined"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
            :aria-required="required || undefined"
            class="min-w-0 w-full border-0 bg-transparent px-1 py-0 text-sm shadow-none outline-none placeholder:text-muted-foreground focus-visible:ring-0 disabled:cursor-not-allowed"
          />
        </div>
        <Button
          v-if="canClear"
          type="button"
          variant="ghost"
          size="icon-xs"
          aria-label="清空选择"
          class="shrink-0"
          @click="clearValue"
        >
          <XIcon class="size-4" aria-hidden="true" />
        </Button>
      </ComboboxAnchor>
      <ComboboxList position="popper">
        <ComboboxEmpty>暂无可选项</ComboboxEmpty>
        <ComboboxGroup>
          <ComboboxItem
            v-for="entry in filteredOptions"
            :key="`${entry.option.value}:${entry.index}`"
            :value="optionToken(entry.index)"
            :text-value="entry.option.label"
            :disabled="entry.option.disabled"
          >
            <span class="min-w-0 truncate">{{ entry.option.label }}</span>
            <ComboboxItemIndicator>
              <CheckIcon class="size-4" aria-hidden="true" />
            </ComboboxItemIndicator>
          </ComboboxItem>
        </ComboboxGroup>
      </ComboboxList>
    </Combobox>
  </div>
</template>
