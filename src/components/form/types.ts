import type { HTMLAttributes } from 'vue'

export type SelectOption = {
  value: string
  label: string
  disabled?: boolean
}

export interface FormControlProps {
  id?: string
  disabled?: boolean
  readonly?: boolean
  invalid?: boolean
  describedBy?: string
  placeholder?: string
  clearable?: boolean
  required?: boolean
  class?: HTMLAttributes['class']
}
