// src/utils/chartAdapter.ts
// 图表配置工厂 (ECharts Option Factory)
//
// 将后端返回的 ChartPayload 转换为 ECharts option 对象。
// 根据 chartType 选择不同的图表配置策略。
//
// 支持的图表类型（与 bi/plugins/*.py 对应）：
//   bar_chart        → 柱状图
//   line_chart       → 折线图
//   scatter_chart    → 散点图
//   pie_chart        → 饼图
//   heatmap_chart    → 热力图
//   boxplot_chart    → 箱线图
//   area_chart       → 面积图（折线+areaStyle）
//   histogram_chart  → 直方图（等同柱状图，bin 由后端处理）
//   density_chart    → 密度分布（折线+平滑）

import type { EChartsOption } from 'echarts'

// ─── 类型（向后兼容：从 chartTypes 导出） ─────────────────────────────────────

export type {
  ColumnInfo,
  ChartPayload,
  DatasetMeta,
  ChartType,
  ChartConfig,
  AxisSide,
} from './chartTypes'
import type { ChartPayload, ChartConfig, AxisSide } from './chartTypes'

// ─── 图表建造器（从 charts/ 子模块导入） ─────────────────────────────────────

import {
  buildBarOption,
  buildLineOption,
  buildScatterOption,
  buildPieOption,
  buildHeatmapOption,
  buildBoxplotOption,
} from './charts/index'

// ─── 主入口函数 ──────────────────────────────────────────────────────────────

/**
 * 将后端 ChartPayload 转换为 ECharts option。
 *
 * @param payload  - 后端返回的图表数据
 * @param config   - 用户在前端选择的图表参数
 * @returns        EChartsOption 对象（可直接传给 <VChart :option="..." />）
 */
export function buildChartOption(
  payload: ChartPayload,
  config: ChartConfig
): EChartsOption {
  const { chartType, xCol, yCol, yCols: rawYCols, yAxisSides: rawSides, swapAxes, colorCol, title } = config
  const rows = payload.rows
  const yCols = (rawYCols && rawYCols.length > 0 ? rawYCols : [yCol]).filter(Boolean)
  const yAxisSides = yCols.map((_, idx) => rawSides?.[idx] ?? 'left') as AxisSide[]
  const primaryY = yCols[0] ?? yCol

  switch (chartType) {
    case 'pie_chart':
      return buildPieOption(rows, xCol, primaryY, title)
    case 'scatter_chart':
      return buildScatterOption(rows, xCol, primaryY, colorCol, title)
    case 'heatmap_chart':
      return buildHeatmapOption(rows, xCol, primaryY, colorCol, title)
    case 'boxplot_chart':
      return buildBoxplotOption(rows, xCol, primaryY, title)
    case 'area_chart':
      return buildLineOption(rows, xCol, yCols, yAxisSides, colorCol, title, true)
    case 'density_chart':
      return buildLineOption(rows, xCol, yCols, yAxisSides, colorCol, title, true, true)
    case 'line_chart':
      return buildLineOption(rows, xCol, yCols, yAxisSides, colorCol, title, false)
    case 'bar_chart':
    case 'histogram_chart':
    default:
      return buildBarOption(rows, xCol, yCols, yAxisSides, colorCol, title, !!swapAxes)
  }
}

