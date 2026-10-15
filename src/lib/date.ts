/** 本地日期工具：due / scheduled_week / focus_on 统一用 YYYY-MM-DD 本地日期串 */

/** 本地日期串 YYYY-MM-DD（按本地时区取值，不用 UTC 免得日期错一天） */
export function formatDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${date.getFullYear()}-${month}-${day}`
}

/** 日期串平移天数（从本地零点起算，不受时区影响） */
export function addDays(date: string, days: number): string {
  const value = new Date(`${date}T00:00:00`)
  value.setDate(value.getDate() + days)
  return formatDate(value)
}

/** 日期串所在周的周一；scheduled_week 统一存周一日期，避免「第几周」歧义 */
export function mondayOf(date: string): string {
  const day = new Date(`${date}T00:00:00`).getDay() // 0 = 周日
  return addDays(date, -((day + 6) % 7))
}
