import { parseDate, parseTime, Time, type CalendarDate } from '@internationalized/date'

const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/
const TIME_PATTERN = /^(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d$/

export function parseCalendarDate(value?: string): CalendarDate | undefined {
  if (!value || !DATE_PATTERN.test(value)) return undefined

  try {
    return parseDate(value)
  } catch {
    return undefined
  }
}

export function parseTimeValue(value?: string): Time | undefined {
  if (!value || !TIME_PATTERN.test(value)) return undefined

  try {
    return parseTime(value)
  } catch {
    return undefined
  }
}

export function isDateWithinRange(
  value: CalendarDate,
  minValue?: CalendarDate,
  maxValue?: CalendarDate,
): boolean {
  return (!minValue || value.compare(minValue) >= 0) && (!maxValue || value.compare(maxValue) <= 0)
}

export function timeToSeconds(value: Pick<Time, 'hour' | 'minute' | 'second'>): number {
  return value.hour * 3600 + value.minute * 60 + value.second
}

export function isTimeWithinRange(
  value: Pick<Time, 'hour' | 'minute' | 'second'>,
  minValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
  maxValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
): boolean {
  const seconds = timeToSeconds(value)

  return (
    (!minValue || seconds >= timeToSeconds(minValue)) &&
    (!maxValue || seconds <= timeToSeconds(maxValue))
  )
}

export function isMinuteWithinRange(
  hour: number,
  minute: number,
  minValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
  maxValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
): boolean {
  const minuteStart = hour * 3600 + minute * 60
  const minuteEnd = minuteStart + 59

  return (
    (!minValue || minuteEnd >= timeToSeconds(minValue)) &&
    (!maxValue || minuteStart <= timeToSeconds(maxValue))
  )
}

export function clampTimeToRange(
  value: Time,
  minValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
  maxValue?: Pick<Time, 'hour' | 'minute' | 'second'>,
): Time {
  const seconds = Math.min(
    maxValue ? timeToSeconds(maxValue) : 86399,
    Math.max(minValue ? timeToSeconds(minValue) : 0, timeToSeconds(value)),
  )

  return new Time(Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60)
}
export function formatTimeValue(value: Pick<Time, 'hour' | 'minute' | 'second'>): string {
  return `${String(value.hour).padStart(2, '0')}:${String(value.minute).padStart(2, '0')}:${String(value.second).padStart(2, '0')}`
}
