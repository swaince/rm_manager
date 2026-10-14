export const layoutTags = [
  'article',
  'aside',
  'div',
  'footer',
  'header',
  'li',
  'main',
  'nav',
  'ol',
  'section',
  'span',
  'ul',
] as const

export type LayoutTag = (typeof layoutTags)[number]
export type RowJustify =
  'start' | 'center' | 'end' | 'space-between' | 'space-around' | 'space-evenly'
export type RowAlign = 'start' | 'center' | 'end' | 'stretch'

const layoutTagSet: ReadonlySet<string> = new Set(layoutTags)

export function resolveLayoutTag(value: unknown): LayoutTag {
  return typeof value === 'string' && layoutTagSet.has(value) ? (value as LayoutTag) : 'div'
}
