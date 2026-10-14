export interface DateTimeControlProps {
  id?: string
  disabled?: boolean
  readonly?: boolean
  invalid?: boolean
  describedBy?: string
  ariaLabel?: string
  ariaLabelledby?: string
  name?: string
  placeholder?: string
  clearable?: boolean
  min?: string
  max?: string
}

export interface DatePickerProps extends Omit<DateTimeControlProps, 'name'> {
  modelValue: string
}

export interface TimePickerProps extends DateTimeControlProps {
  modelValue: string
}
