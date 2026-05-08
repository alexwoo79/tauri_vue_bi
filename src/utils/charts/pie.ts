// src/utils/charts/pie.ts
// 饼图建造器

import type { EChartsOption } from 'echarts'
import type { ChartPayload } from '../chartTypes'
import { baseOption } from './base'

export function buildPieOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  title?: string,
): EChartsOption {
  return {
    ...baseOption(title),
    tooltip: { trigger: 'item' },
    series: [
      {
        type: 'pie',
        radius: ['35%', '65%'],
        data: rows.map((r) => ({
          name: String(r[xCol] ?? ''),
          value: Number(r[yCol] ?? 0),
        })),
        emphasis: { itemStyle: { shadowBlur: 10 } },
      },
    ],
  }
}
