// src/utils/chartTypes.ts
// 共享类型定义（供 chartAdapter.ts 与 charts/ 子模块共用）

// ─── 后端数据类型（与 Rust ApiResult<ChartPayload> 对应） ─────────────────────

export interface ColumnInfo {
  name: string
  dtype: string
}

export interface ChartPayload {
  columns: ColumnInfo[]
  rows: Record<string, string | number | null>[]
  total_rows: number
  notices?: string[]
}

export interface DatasetMeta {
  id: string
  name: string
  source: string
  total_rows: number
  total_cols: number
  created_at_ms: number
}

// ─── 图表类型枚举 ────────────────────────────────────────────────────────────

export type ChartType =
  | 'bar_chart'
  | 'line_chart'
  | 'scatter_chart'
  | 'pie_chart'
  | 'heatmap_chart'
  | 'boxplot_chart'
  | 'area_chart'
  | 'histogram_chart'
  | 'density_chart'

// ─── 图表参数 ────────────────────────────────────────────────────────────────

export interface ChartConfig {
  chartType: ChartType
  xCol: string
  yCol: string
  yCols?: string[]
  yAxisSides?: Array<'left' | 'right'>
  swapAxes?: boolean
  colorCol?: string
  title?: string
}

export type AxisSide = 'left' | 'right'
