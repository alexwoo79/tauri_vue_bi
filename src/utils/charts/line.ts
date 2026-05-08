// src/utils/charts/line.ts
// 折线图 / 面积图 / 密度图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload, AxisSide } from '../chartTypes'
import { baseOption } from './base'
import { buildDualYAxis } from './bar'

export function buildLineOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCols: string[],
  yAxisSides: AxisSide[],
  colorCol?: string,
  title?: string,
  showArea = false,
  smooth = false,
): EChartsOption {
  const base = baseOption(title)
  const areaStyle = showArea ? {} : undefined
  const xValues = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
  const { yAxis, axisIndexFor } = buildDualYAxis(yCols, yAxisSides)

  if (colorCol) {
    const groups = Array.from(new Set(rows.map((r) => String(r[colorCol] ?? ''))))
    const series = yCols.flatMap((y) =>
      groups.map((g) => ({
        name: yCols.length > 1 ? `${y} · ${g}` : g,
        type: 'line' as const,
        smooth,
        areaStyle,
        yAxisIndex: axisIndexFor(y),
        data: xValues.map((x) => {
          const row = rows.find((r) => String(r[xCol]) === x && String(r[colorCol]) === g)
          return row ? (row[y] ?? null) : null
        }),
      }))
    )
    return { ...base, xAxis: { type: 'category', data: xValues }, yAxis, series }
  }

  return {
    ...base,
    xAxis: { type: 'category', data: xValues },
    yAxis,
    series: yCols.map((y) => ({
      name: y,
      type: 'line' as const,
      smooth,
      areaStyle,
      yAxisIndex: axisIndexFor(y),
      data: xValues.map((x) => {
        const row = rows.find((r) => String(r[xCol]) === x)
        return row ? (row[y] ?? 0) : 0
      }),
    })),
  }
}
