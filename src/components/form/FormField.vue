<script setup lang="ts">
import { computed, useId } from 'vue'
import { Field, FieldDescription, FieldError, FieldLabel } from '@/components/ui/field'

const props = withDefaults(
  defineProps<{
    label: string
    id?: string
    required?: boolean
    hint?: string
    error?: string
  }>(),
  {
    required: false,
  },
)

const generatedId = useId()
const fieldId = computed(() => props.id?.trim() || `form-field-${generatedId}`)
const invalid = computed(() => Boolean(props.error))
const hintId = computed(() => `${fieldId.value}-hint`)
const errorId = computed(() => `${fieldId.value}-error`)
const describedBy = computed(() => {
  const ids = []

  if (props.hint) ids.push(hintId.value)
  if (props.error) ids.push(errorId.value)

  return ids.length ? ids.join(' ') : undefined
})
</script>

<template>
  <Field :data-invalid="invalid || undefined" class="w-full min-w-0">
    <FieldLabel :id="`${fieldId}-label`" :for="fieldId">
      {{ label }}
      <span v-if="required" aria-hidden="true" class="ml-0.5 text-destructive">*</span>
    </FieldLabel>
    <slot
      :id="fieldId"
      :invalid="invalid"
      :described-by="describedBy"
      :labelled-by="`${fieldId}-label`"
      :required="required"
    />
    <FieldDescription v-if="hint" :id="hintId">
      {{ hint }}
    </FieldDescription>
    <FieldError v-if="error" :id="errorId">
      {{ error }}
    </FieldError>
  </Field>
</template>
