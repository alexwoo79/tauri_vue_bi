// src/utils/charts/scatter.ts
// 散点图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload } from '../chartTypes'
import { baseOption } from './base'

export function buildScatterOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  colorCol?: string,
  title?: string,
): EChartsOption {
  const base = baseOption(title)

  if (colorCol) {
    const groups = Array.from(new Set(rows.map((r) => String(r[colorCol] ?? ''))))
    const series = groups.map((g) => ({
      name: g,
      type: 'scatter' as const,
      data: rows
        .filter((r) => String(r[colorCol]) === g)
        .map((r) => [r[xCol] ?? 0, r[yCol] ?? 0]),
    }))
    return { ...base, xAxis: { type: 'value' }, yAxis: { type: 'value' }, series }
  }

  return {
    ...base,
    xAxis: { type: 'value' },
    yAxis: { type: 'value' },
    series: [{ type: 'scatter', data: rows.map((r) => [r[xCol] ?? 0, r[yCol] ?? 0]) }],
  }
}
