// src/utils/charts/heatmap.ts
// 热力图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload } from '../chartTypes'
import { baseOption } from './base'

export function buildHeatmapOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  valueCol?: string,
  title?: string,
): EChartsOption {
  const xValues = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
  const yValues = Array.from(new Set(rows.map((r) => String(r[yCol] ?? ''))))
  const vCol = valueCol ?? yCol

  const data = rows.map((r) => [
    xValues.indexOf(String(r[xCol] ?? '')),
    yValues.indexOf(String(r[yCol] ?? '')),
    r[vCol] ?? 0,
  ])

  return {
    ...baseOption(title),
    tooltip: { position: 'top' },
    xAxis: { type: 'category', data: xValues },
    yAxis: { type: 'category', data: yValues },
    visualMap: {
      min: 0,
      max: Math.max(...rows.map((r) => Number(r[vCol] ?? 0))),
      calculable: true,
    },
    series: [{ type: 'heatmap', data, label: { show: true } }],
  }
}
