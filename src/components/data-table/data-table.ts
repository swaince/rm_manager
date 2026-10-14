export type DataTableAlign = 'left' | 'center' | 'right'

export interface DataTableColumn {
  key: string
  label: string
  align?: DataTableAlign
  width?: string
}

export type DataTableRow = Record<string, unknown>
