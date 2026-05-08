// src/utils/charts/boxplot.ts
// 箱线图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload } from '../chartTypes'
import { baseOption } from './base'

export function buildBoxplotOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  title?: string,
): EChartsOption {
  const groups = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
  const boxData = groups.map((g) => {
    const vals = rows
      .filter((r) => String(r[xCol]) === g)
      .map((r) => Number(r[yCol] ?? 0))
      .sort((a, b) => a - b)
    if (vals.length === 0) return [0, 0, 0, 0, 0]
    const q1 = vals[Math.floor(vals.length * 0.25)]
    const q2 = vals[Math.floor(vals.length * 0.5)]
    const q3 = vals[Math.floor(vals.length * 0.75)]
    return [vals[0], q1, q2, q3, vals[vals.length - 1]]
  })

  return {
    ...baseOption(title),
    xAxis: { type: 'category', data: groups },
    yAxis: { type: 'value' },
    series: [{ type: 'boxplot', data: boxData }],
  }
}
