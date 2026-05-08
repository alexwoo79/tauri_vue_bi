// src/utils/charts/bar.ts
// 柱状图 / 水平条形图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload, AxisSide } from '../chartTypes'
import { baseOption } from './base'

const barWidthConstraint = { barMinWidth: 12, barMaxWidth: 26 }

export function buildDualYAxis(yCols: string[], yAxisSides: AxisSide[]) {
  const hasRight = yCols.some((_, idx) => yAxisSides[idx] === 'right')
  if (!hasRight) {
    return {
      yAxis: { type: 'value' as const },
      axisIndexFor: (_y: string) => 0,
    }
  }

  const leftNames = yCols.filter((_, idx) => yAxisSides[idx] !== 'right')
  const rightNames = yCols.filter((_, idx) => yAxisSides[idx] === 'right')

  return {
    yAxis: [
      { type: 'value' as const, position: 'left' as const, name: leftNames.join(' / ') || '值' },
      { type: 'value' as const, position: 'right' as const, name: rightNames.join(' / ') || '值' },
    ],
    axisIndexFor: (y: string) => {
      const idx = yCols.indexOf(y)
      if (idx < 0) return 0
      return yAxisSides[idx] === 'right' ? 1 : 0
    },
  }
}

export function buildBarOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCols: string[],
  yAxisSides: AxisSide[],
  colorCol?: string,
  title?: string,
  horizontal = false,
): EChartsOption {
  const base = baseOption(title)
  const xValues = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
  const { yAxis, axisIndexFor } = buildDualYAxis(yCols, yAxisSides)

  if (colorCol) {
    const groups = Array.from(new Set(rows.map((r) => String(r[colorCol] ?? ''))))
    const series = yCols.flatMap((y) =>
      groups.map((g) => ({
        name: yCols.length > 1 ? `${y} · ${g}` : g,
        type: 'bar' as const,
        yAxisIndex: axisIndexFor(y),
        ...barWidthConstraint,
        data: xValues.map((x) => {
          const row = rows.find((r) => String(r[xCol]) === x && String(r[colorCol]) === g)
          return row ? (row[y] ?? 0) : 0
        }),
      }))
    )
    if (horizontal) {
      return {
        ...base,
        xAxis: { type: 'value' },
        yAxis: { type: 'category', data: xValues },
        series,
      }
    }
    return { ...base, xAxis: { type: 'category', data: xValues }, yAxis, series }
  }

  if (horizontal) {
    return {
      ...base,
      xAxis: { type: 'value' },
      yAxis: { type: 'category', data: xValues },
      series: yCols.map((y) => ({
        name: y,
        type: 'bar' as const,
        ...barWidthConstraint,
        data: xValues.map((x) => {
          const row = rows.find((r) => String(r[xCol]) === x)
          return row ? (row[y] ?? 0) : 0
        }),
      })),
    }
  }

  return {
    ...base,
    xAxis: { type: 'category', data: xValues },
    yAxis,
    series: yCols.map((y) => ({
      name: y,
      type: 'bar' as const,
      yAxisIndex: axisIndexFor(y),
      ...barWidthConstraint,
      data: xValues.map((x) => {
        const row = rows.find((r) => String(r[xCol]) === x)
        return row ? (row[y] ?? 0) : 0
      }),
    })),
  }
}
