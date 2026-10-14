<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  Combobox,
  ComboboxAnchor,
  ComboboxEmpty,
  ComboboxGroup,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from '@/components/ui/combobox'
import { XIcon } from '@lucide/vue'
import Button from '@/components/ui/button/Button.vue'
import { cn } from '@/lib/utils'
import type { SelectOption } from './types'

defineOptions({ name: 'FormAutocomplete', inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    modelValue?: string
    options: SelectOption[]
    id?: string
    disabled?: boolean
    readonly?: boolean
    invalid?: boolean
    describedBy?: string
    placeholder?: string
    clearable?: boolean
    required?: boolean
    loading?: boolean
    error?: string
    class?: string
  }>(),
  {
    modelValue: '',
    disabled: false,
    readonly: false,
    invalid: false,
    clearable: false,
    required: false,
    loading: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  search: [query: string]
}>()

const inputValue = ref(props.modelValue)
const selectedToken = ref<string>()
const tokenOptions = computed(
  () => new Map(props.options.map((option, index) => [`form-option-${index}`, option])),
)
const canClear = computed(
  () => props.clearable && !!inputValue.value && !props.disabled && !props.readonly,
)

watch(
  () => [props.modelValue, props.options] as const,
  ([value, options]) => {
    inputValue.value = options.find((option) => option.value === value)?.label ?? value
  },
  { immediate: true },
)

function onValueChange(value: string) {
  if (props.disabled || props.readonly) return
  inputValue.value = value
  selectedToken.value = undefined
  emit('update:modelValue', value)
  emit('search', value)
}

function selectOption(value: unknown) {
  if (props.disabled || props.readonly || typeof value !== 'string') return
  const option = tokenOptions.value.get(value)
  if (!option || option.disabled) return
  selectedToken.value = value
  inputValue.value = option.label
  emit('update:modelValue', option.value)
}

function clearValue() {
  inputValue.value = ''
  emit('update:modelValue', '')
  emit('search', '')
}
</script>

<template>
  <div class="relative w-full min-w-0">
    <Combobox
      :model-value="selectedToken"
      :reset-search-term-on-blur="false"
      :reset-search-term-on-select="false"
      :open="readonly ? false : undefined"
      :disabled="disabled"
      :required="required"
      :ignore-filter="true"
      :open-on-focus="true"
      @update:model-value="selectOption"
    >
      <ComboboxAnchor class="w-full min-w-0">
        <ComboboxInput
          :model-value="inputValue"
          @update:model-value="onValueChange"
          v-bind="$attrs"
          :id="id"
          :placeholder="placeholder"
          :disabled="disabled"
          :readonly="readonly"
          :aria-readonly="readonly || undefined"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
          :aria-required="required || undefined"
          :class="cn('w-full min-w-0', canClear && 'pr-9', props.class)"
        />
      </ComboboxAnchor>
      <ComboboxList position="popper">
        <div v-if="loading" role="status" class="px-3 py-2 text-muted-foreground">加载中…</div>
        <div v-else-if="error" role="alert" class="px-3 py-2 text-destructive">
          {{ error }}
        </div>
        <ComboboxEmpty v-else> 暂无建议 </ComboboxEmpty>
        <ComboboxGroup v-if="!loading && !error">
          <ComboboxItem
            v-for="(option, index) in options"
            :key="`${option.value}:${index}`"
            :value="`form-option-${index}`"
            :text-value="option.label"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </ComboboxItem>
        </ComboboxGroup>
      </ComboboxList>
    </Combobox>
    <Button
      v-if="canClear"
      type="button"
      variant="ghost"
      size="icon-xs"
      aria-label="清空输入"
      class="absolute right-1 top-1/2 z-10 -translate-y-1/2"
      @click="clearValue"
    >
      <XIcon class="size-4" aria-hidden="true" />
    </Button>
  </div>
</template>
