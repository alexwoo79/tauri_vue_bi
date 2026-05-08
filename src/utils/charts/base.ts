// src/utils/charts/base.ts
// ECharts 公共基础 option —— 工具栏、grid、数据视图等

import type { EChartsOption } from 'echarts'

export function baseOption(title?: string): Partial<EChartsOption> {
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
            const esc = (value: unknown) =>
              String(value == null ? '' : value)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')

            const toRows = (o: any): string[][] => {
              const xAxisData = o?.xAxis?.data ?? o?.xAxis?.[0]?.data ?? []
              const seriesList = Array.isArray(o?.series) ? o.series : []

              if (Array.isArray(xAxisData) && xAxisData.length > 0 && seriesList.length > 0) {
                const headers = [
                  'X',
                  ...seriesList.map((s: any, i: number) => String(s?.name ?? `系列${i + 1}`)),
                ]
                const body = xAxisData.map((x: any, idx: number) => {
                  const row = [x]
                  for (const s of seriesList) {
                    const v = Array.isArray(s?.data) ? s.data[idx] : ''
                    if (Array.isArray(v)) row.push(v.join(', '))
                    else if (v && typeof v === 'object') row.push((v as any).value ?? JSON.stringify(v))
                    else row.push(v ?? '')
                  }
                  return row.map((cell) => esc(cell))
                })
                return [headers.map((h) => esc(h)), ...body]
              }

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
            const body = rows
              .slice(1)
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
