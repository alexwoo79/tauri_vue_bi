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

// ─── 后端数据类型（与 Rust ApiResult<ChartPayload> 对应） ─────────────────────

export interface ColumnInfo {
  name: string
  dtype: string
}

export interface ChartPayload {
  columns: ColumnInfo[]
  rows: Record<string, string | number | null>[]
  total_rows: number
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
  colorCol?: string
  title?: string
}

// ─── 公共基础 option ─────────────────────────────────────────────────────────

function baseOption(title?: string): Partial<EChartsOption> {
  return {
    title: title ? { text: title, left: 'center', textStyle: { fontSize: 14 } } : undefined,
    tooltip: { trigger: 'axis' as const },
    legend: { bottom: 0 },
    toolbox: {
      feature: {
        dataZoom: { title: { zoom: '区域缩放', back: '还原' } },
        restore: { title: '还原' },
        dataView: {
          title: '数据视图',
          lang: ['数据视图', '关闭', '刷新'],
          readOnly: true,
          optionToContent: (opt: any) => {
            const esc = (value: unknown) => String(value == null ? '' : value)
              .replace(/&/g, '&amp;')
              .replace(/</g, '&lt;')
              .replace(/>/g, '&gt;')

            const toRows = (o: any): string[][] => {
              const xAxisData = o?.xAxis?.data
                ?? o?.xAxis?.[0]?.data
                ?? []
              const seriesList = Array.isArray(o?.series) ? o.series : []

              // 笛卡尔图：按 x 轴展开为表格
              if (Array.isArray(xAxisData) && xAxisData.length > 0 && seriesList.length > 0) {
                const headers = ['X', ...seriesList.map((s: any, i: number) => String(s?.name ?? `系列${i + 1}`))]
                const body = xAxisData.map((x: any, idx: number) => {
                  const row = [x]
                  for (const s of seriesList) {
                    const v = Array.isArray(s?.data) ? s.data[idx] : ''
                    if (Array.isArray(v)) row.push(v.join(', '))
                    else if (v && typeof v === 'object') row.push((v as any).value ?? JSON.stringify(v))
                    else row.push(v ?? '')
                  }
                  return row.map(cell => esc(cell))
                })
                return [headers.map(h => esc(h)), ...body]
              }

              // 饼图/散点/热力图等：从 series.data 兜底
              const rows: string[][] = [['系列', '名称', '值']]
              for (let i = 0; i < seriesList.length; i += 1) {
                const s = seriesList[i]
                const sName = String(s?.name ?? `系列${i + 1}`)
                const data = Array.isArray(s?.data) ? s.data : []
                for (const item of data) {
                  if (Array.isArray(item)) {
                    rows.push([esc(sName), '', esc(item.join(', '))])
                  } else if (item && typeof item === 'object') {
                    rows.push([
                      esc(sName),
                      esc((item as any).name ?? ''),
                      esc((item as any).value ?? JSON.stringify(item)),
                    ])
                  } else {
                    rows.push([esc(sName), '', esc(item ?? '')])
                  }
                }
              }
              return rows
            }

            const rows = toRows(opt)
            const head = rows[0] ?? []
            const body = rows.slice(1)
              .map((r) => `<tr><td>${r.join('</td><td>')}</td></tr>`)
              .join('')

            return `<style>.dv-wrap{padding:12px 16px;font-family:sans-serif;font-size:13px;color:#1a1a2e}.dv-table{border-collapse:collapse;width:100%;min-width:640px}.dv-table th{background:#eef3ff;color:#334155;font-weight:600;padding:7px 10px;border:1px solid #cfd8ea;text-align:left;white-space:nowrap}.dv-table td{padding:6px 10px;border:1px solid #d9e2f2;vertical-align:top;background:#ffffff}.dv-table tr:nth-child(even) td{background:#f7faff}</style><div class="dv-wrap"><table class="dv-table"><thead><tr><th>${head.join('</th><th>')}</th></tr></thead><tbody>${body}</tbody></table></div>`
          },
        },
        saveAsImage: { title: '保存图片' },
      },
    },
    grid: { left: 60, right: 40, top: 40, bottom: 60, containLabel: true },
  }
}

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
  const { chartType, xCol, yCol, colorCol, title } = config
  const rows = payload.rows

  switch (chartType) {
    case 'pie_chart':
      return buildPieOption(rows, xCol, yCol, title)
    case 'scatter_chart':
      return buildScatterOption(rows, xCol, yCol, colorCol, title)
    case 'heatmap_chart':
      return buildHeatmapOption(rows, xCol, yCol, colorCol, title)
    case 'boxplot_chart':
      return buildBoxplotOption(rows, xCol, yCol, title)
    case 'area_chart':
      return buildLineOption(rows, xCol, yCol, colorCol, title, true)
    case 'density_chart':
      return buildLineOption(rows, xCol, yCol, colorCol, title, true, true)
    case 'line_chart':
      return buildLineOption(rows, xCol, yCol, colorCol, title, false)
    case 'bar_chart':
    case 'histogram_chart':
    default:
      return buildBarOption(rows, xCol, yCol, colorCol, title)
  }
}

// ─── 柱状图 ──────────────────────────────────────────────────────────────────

function buildBarOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  colorCol?: string,
  title?: string
): EChartsOption {
  const base = baseOption(title)

  if (colorCol) {
    // 按 colorCol 分组，生成多系列
    const groups = Array.from(new Set(rows.map((r) => String(r[colorCol] ?? ''))))
    const xValues = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
    const series = groups.map((g) => ({
      name: g,
      type: 'bar' as const,
      data: xValues.map((x) => {
        const row = rows.find((r) => String(r[xCol]) === x && String(r[colorCol]) === g)
        return row ? (row[yCol] ?? 0) : 0
      }),
    }))
    return { ...base, xAxis: { type: 'category', data: xValues }, yAxis: { type: 'value' }, series }
  }

  return {
    ...base,
    xAxis: { type: 'category', data: rows.map((r) => String(r[xCol] ?? '')) },
    yAxis: { type: 'value' },
    series: [{ type: 'bar', data: rows.map((r) => r[yCol] ?? 0) }],
  }
}

// ─── 折线图 / 面积图 / 密度图 ─────────────────────────────────────────────────

function buildLineOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  colorCol?: string,
  title?: string,
  showArea = false,
  smooth = false
): EChartsOption {
  const base = baseOption(title)
  const areaStyle = showArea ? {} : undefined

  if (colorCol) {
    const groups = Array.from(new Set(rows.map((r) => String(r[colorCol] ?? ''))))
    const xValues = Array.from(new Set(rows.map((r) => String(r[xCol] ?? ''))))
    const series = groups.map((g) => ({
      name: g,
      type: 'line' as const,
      smooth,
      areaStyle,
      data: xValues.map((x) => {
        const row = rows.find((r) => String(r[xCol]) === x && String(r[colorCol]) === g)
        return row ? (row[yCol] ?? null) : null
      }),
    }))
    return { ...base, xAxis: { type: 'category', data: xValues }, yAxis: { type: 'value' }, series }
  }

  return {
    ...base,
    xAxis: { type: 'category', data: rows.map((r) => String(r[xCol] ?? '')) },
    yAxis: { type: 'value' },
    series: [{ type: 'line', smooth, areaStyle, data: rows.map((r) => r[yCol] ?? 0) }],
  }
}

// ─── 散点图 ──────────────────────────────────────────────────────────────────

function buildScatterOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  colorCol?: string,
  title?: string
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

// ─── 饼图 ────────────────────────────────────────────────────────────────────

function buildPieOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  title?: string
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

// ─── 热力图 ──────────────────────────────────────────────────────────────────

function buildHeatmapOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  valueCol?: string,
  title?: string
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
    visualMap: { min: 0, max: Math.max(...rows.map((r) => Number(r[vCol] ?? 0))), calculable: true },
    series: [{ type: 'heatmap', data, label: { show: true } }],
  }
}

// ─── 箱线图 ──────────────────────────────────────────────────────────────────

function buildBoxplotOption(
  rows: ChartPayload['rows'],
  xCol: string,
  yCol: string,
  title?: string
): EChartsOption {
  // Group data by xCol and compute box statistics per group
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
