<script setup lang="ts">
import { computed } from 'vue'
import type { EChartsOption } from 'echarts'
import BiChart from './BiChart.vue'
import { useDataStore } from '../stores/dataStore'
import { getThemeProfile } from '../utils/echartsTheme'

interface GanttRow {
    [key: string]: string | number | null
}

interface GanttOptions {
    showTaskDetails?: boolean
    showDuration?: boolean
    sortByStart?: boolean
    autoNumber?: boolean
    darkTheme?: boolean
    granularity?: 'day' | 'week' | 'month' | 'quarter' | 'year'
    /** 横道上显示的内容：none=不显示 name=任务名 duration=天数 dates=日期区间 nameAndDuration=名称+天数 detail=详情列 */
    barLabel?: 'none' | 'name' | 'duration' | 'dates' | 'nameAndDuration' | 'detail'
}

interface Props {
    rows: GanttRow[]
    taskCol: string
    startCol: string
    endCol: string
    projectCol?: string
    colorCol?: string
    milestoneCol?: string
    detailCol?: string
    loading?: boolean
    height?: string
    options?: GanttOptions
}

interface TimelineRow {
    rowType: 'project' | 'task'
    rowLabel: string
    taskName: string
    projectName: string
    colorGroup: string
    start: number
    end: number
    durationDays: number
    detail: string
    sourceRow: GanttRow
}

const props = withDefaults(defineProps<Props>(), {
    loading: false,
    height: '480px',
})

const dataStore = useDataStore()

const DAY_MS = 24 * 3600 * 1000
const NOW_TS = Date.now()

function floorToDay(ts: number) {
    const d = new Date(ts)
    return +new Date(d.getFullYear(), d.getMonth(), d.getDate())
}

function ceilToDay(ts: number) {
    const d = new Date(ts)
    return +new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1)
}

function floorToWeek(ts: number) {
    const d = new Date(floorToDay(ts))
    const day = d.getDay()
    d.setDate(d.getDate() - day)
    return +d
}

function ceilToWeek(ts: number) {
    const d = new Date(floorToDay(ts))
    const day = d.getDay()
    d.setDate(d.getDate() + (day === 0 ? 0 : 7 - day))
    return +d
}

function floorToMonth(ts: number) {
    const d = new Date(ts)
    return +new Date(d.getFullYear(), d.getMonth(), 1)
}

function ceilToMonth(ts: number) {
    const d = new Date(ts)
    return +new Date(d.getFullYear(), d.getMonth() + 1, 1)
}

function floorToQuarter(ts: number) {
    const d = new Date(ts)
    const q = Math.floor(d.getMonth() / 3)
    return +new Date(d.getFullYear(), q * 3, 1)
}

function ceilToQuarter(ts: number) {
    const d = new Date(ts)
    const q = Math.floor(d.getMonth() / 3)
    return +new Date(d.getFullYear(), (q + 1) * 3, 1)
}

function floorToYear(ts: number) {
    return +new Date(new Date(ts).getFullYear(), 0, 1)
}

function ceilToYear(ts: number) {
    return +new Date(new Date(ts).getFullYear() + 1, 0, 1)
}

function floorTo(ts: number, granularity: string) {
    switch (granularity) {
        case 'day': return floorToDay(ts)
        case 'week': return floorToWeek(ts)
        case 'quarter': return floorToQuarter(ts)
        case 'year': return floorToYear(ts)
        default: return floorToMonth(ts)
    }
}

function ceilTo(ts: number, granularity: string) {
    switch (granularity) {
        case 'day': return ceilToDay(ts)
        case 'week': return ceilToWeek(ts)
        case 'quarter': return ceilToQuarter(ts)
        case 'year': return ceilToYear(ts)
        default: return ceilToMonth(ts)
    }
}

function minIntervalFor(granularity: string) {
    switch (granularity) {
        case 'day': return DAY_MS
        case 'week': return 7 * DAY_MS
        case 'quarter': return 89 * DAY_MS
        case 'year': return 365 * DAY_MS
        default: return 28 * DAY_MS
    }
}

function buildGanttDataViewTable(rows: GanttRow[], taskCol: string, startCol: string, endCol: string, showDuration: boolean, projectCol?: string, detailCol?: string) {
    const esc = (value: unknown) => String(value == null ? '' : value)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')

    const fmt = (value: unknown) => {
        const ts = new Date(String(value ?? '')).getTime()
        return Number.isFinite(ts) ? new Date(ts).toLocaleDateString('zh-CN') : ''
    }

    const headCols = projectCol ? ['项目', '任务', '开始', '结束'] : ['任务', '开始', '结束']
    if (showDuration) headCols.push('周期(天)')
    if (detailCol) headCols.push(detailCol)

    const body = rows.map((row) => {
        const start = new Date(String(row[startCol] ?? '')).getTime()
        const end = new Date(String(row[endCol] ?? '')).getTime()
        const duration = Number.isFinite(start) && Number.isFinite(end)
            ? Math.max(0, Math.round((end - start) / DAY_MS))
            : ''

        const cols = [
            ...(projectCol ? [esc(row[projectCol])] : []),
            esc(row[taskCol]),
            esc(fmt(row[startCol])),
            esc(fmt(row[endCol])),
        ]
        if (showDuration) cols.push(esc(duration))
        if (detailCol) cols.push(esc(row[detailCol]))
        return `<tr><td>${cols.join('</td><td>')}</td></tr>`
    }).join('')

    return `<style>.dv-wrap{padding:12px 16px;font-family:sans-serif;font-size:13px;color:#1a1a2e}.dv-table{border-collapse:collapse;width:100%;min-width:640px}.dv-table th{background:#eef3ff;color:#334155;font-weight:600;padding:7px 10px;border:1px solid #cfd8ea;text-align:left;white-space:nowrap}.dv-table td{padding:6px 10px;border:1px solid #d9e2f2;vertical-align:top;background:#ffffff}.dv-table tr:nth-child(even) td{background:#f7faff}</style><div class="dv-wrap"><table class="dv-table"><thead><tr><th>${headCols.join('</th><th>')}</th></tr></thead><tbody>${body}</tbody></table></div>`
}

function buildTimelineRows(rows: GanttRow[]): TimelineRow[] {
    const sortByStart = props.options?.sortByStart !== false
    const autoNumber = props.options?.autoNumber !== false
    const projectBuckets = new Map<string, GanttRow[]>()

    for (const row of rows) {
        const projectName = props.projectCol
            ? String(row[props.projectCol] ?? '').trim() || '未分组'
            : props.colorCol
                ? String(row[props.colorCol] ?? '').trim() || '未分组'
                : '未分组'
        const bucket = projectBuckets.get(projectName) ?? []
        bucket.push(row)
        projectBuckets.set(projectName, bucket)
    }

    const projectEntries = Array.from(projectBuckets.entries()).map(([projectName, items]) => {
        const starts = items.map((row) => new Date(String(row[props.startCol] ?? '')).getTime()).filter(Number.isFinite) as number[]
        const ends = items.map((row) => new Date(String(row[props.endCol] ?? '')).getTime()).filter(Number.isFinite) as number[]
        return {
            projectName,
            items,
            minStart: starts.length > 0 ? Math.min(...starts) : 0,
            maxEnd: ends.length > 0 ? Math.max(...ends) : 0,
        }
    })

    projectEntries.sort((left, right) => {
        if (sortByStart && left.minStart !== right.minStart) return left.minStart - right.minStart
        return left.projectName.localeCompare(right.projectName, 'zh-CN')
    })

    const output: TimelineRow[] = []
    for (const project of projectEntries) {
        const projectColorGroup = props.colorCol
            ? String(project.items[0]?.[props.colorCol] ?? project.projectName)
            : project.projectName

        output.push({
            rowType: 'project',
            rowLabel: project.projectName,
            taskName: project.projectName,
            projectName: project.projectName,
            colorGroup: projectColorGroup,
            start: project.minStart,
            end: project.maxEnd,
            durationDays: Math.max(0, Math.round((project.maxEnd - project.minStart) / DAY_MS)),
            detail: '',
            sourceRow: project.items[0] ?? {},
        })

        const sortedTasks = project.items.slice().sort((left, right) => {
            if (sortByStart) {
                const leftStart = new Date(String(left[props.startCol] ?? '')).getTime()
                const rightStart = new Date(String(right[props.startCol] ?? '')).getTime()
                if (leftStart !== rightStart) return leftStart - rightStart
            }
            return String(left[props.taskCol] ?? '').localeCompare(String(right[props.taskCol] ?? ''), 'zh-CN')
        })

        sortedTasks.forEach((row, index) => {
            const start = new Date(String(row[props.startCol] ?? '')).getTime()
            const end = new Date(String(row[props.endCol] ?? '')).getTime()
            const taskName = String(row[props.taskCol] ?? '')
            const detail = props.detailCol ? String(row[props.detailCol] ?? '') : ''
            const prefix = autoNumber ? `${String(index + 1).padStart(2, '0')}  ` : '  '
            output.push({
                rowType: 'task',
                rowLabel: `${prefix}${taskName}`,
                taskName,
                projectName: project.projectName,
                colorGroup: props.colorCol ? String(row[props.colorCol] ?? projectColorGroup) : projectColorGroup,
                start,
                end,
                durationDays: Math.max(0, Math.round((end - start) / DAY_MS)),
                detail,
                sourceRow: row,
            })
        })
    }

    return output.filter((row) => Number.isFinite(row.start) && Number.isFinite(row.end))
}

const chartOption = computed<EChartsOption | null>(() => {
    if (!props.rows || props.rows.length === 0) return null

    const granularity = props.options?.granularity ?? 'month'
    const showDuration = props.options?.showDuration !== false
    const showTaskDetails = props.options?.showTaskDetails !== false
    const barLabel = props.options?.barLabel ?? 'none'
    const timelineRows = buildTimelineRows(props.rows)
    if (timelineRows.length === 0) return null

    // 从当前主题取调色盘和颜色配置
    const profile = getThemeProfile(dataStore.currentTheme)
    const palette = profile.palette.length > 0 ? profile.palette : ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc']
    const axisLabelColor = profile.axisLabelColor
    const axisLineColor = profile.axisLineColor
    const splitLineColor = profile.splitLineColor
    const timeGridLineColor = 'rgba(148,163,184,0.35)'
    const barTextColor = '#ffffff'
    const detailLabel = String(props.detailCol ?? '详情')

    const taskLabels = timelineRows.map((row) => row.rowLabel)
    const colorGroups = Array.from(new Set(timelineRows.map((row) => row.colorGroup)))
    const colorMap = Object.fromEntries(colorGroups.map((group, index) => [group, palette[index % palette.length]])) as Record<string, string>

    const barData = timelineRows.map((row, index) => ({
        name: row.taskName,
        // value: [idx, start, end, task, project, duration, detail, rowType, color, barLabelMode, textColor, showDuration, showTaskDetails, detailLabel]
        value: [
            index,
            row.start,
            row.end,
            row.taskName,
            row.projectName,
            row.durationDays,
            row.detail,
            row.rowType,
            colorMap[row.colorGroup] ?? palette[0],
            barLabel,
            barTextColor,
            showDuration ? 1 : 0,
            showTaskDetails ? 1 : 0,
            detailLabel,
        ],
    }))

    const milestoneData = props.milestoneCol
        ? timelineRows
            .filter((row) => row.rowType === 'task')
            .filter((row) => row.sourceRow[props.milestoneCol!] != null && String(row.sourceRow[props.milestoneCol!] ?? '').trim() !== '')
            .map((row) => ({
                name: String(row.sourceRow[props.milestoneCol!]),
                value: [taskLabels.indexOf(row.rowLabel), new Date(String(row.sourceRow[props.startCol] ?? '')).getTime()],
                symbol: 'diamond',
                symbolSize: 14,
                itemStyle: { color: '#ffd700' },
            }))
        : []

    const starts = timelineRows.map((row) => row.start)
    const ends = timelineRows.map((row) => row.end)
    const axisMin = floorTo(Math.min(Math.min(...starts), NOW_TS), granularity)
    const axisMax = ceilTo(Math.max(Math.max(...ends), NOW_TS), granularity)
    const firstLabel = taskLabels[0] ?? ''
    const lastLabel = taskLabels[taskLabels.length - 1] ?? ''
    const dataViewHtml = buildGanttDataViewTable(
        props.rows,
        props.taskCol,
        props.startCol,
        props.endCol,
        showDuration,
        props.projectCol,
        props.detailCol,
    )
    // 导出 HTML 时不能依赖组件闭包变量，这里使用“自包含函数”返回固定 HTML。
    const dataViewOptionToContent = new Function(
        `return ${JSON.stringify(dataViewHtml)}`,
    ) as () => string
    // 生成“自包含”时间格式化函数，避免导出 HTML 时依赖组件闭包变量
    const timeLabelFormatter = new Function(
        'value',
        `
        const d = new Date(value)
        const y = d.getFullYear()
        const mm = String(d.getMonth() + 1).padStart(2, '0')
        const dd = String(d.getDate()).padStart(2, '0')
        const g = '${granularity}'
        if (g === 'day') return mm + '-' + dd
        if (g === 'month') return y + '-' + mm
        if (g === 'week') {
            const dt = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()))
            const dayNum = dt.getUTCDay() || 7
            dt.setUTCDate(dt.getUTCDate() + 4 - dayNum)
            const yearStart = new Date(Date.UTC(dt.getUTCFullYear(), 0, 1))
            const weekNo = Math.ceil((((dt.getTime() - yearStart.getTime()) / 86400000) + 1) / 7)
            return y + '-W' + String(weekNo).padStart(2, '0')
        }
        if (g === 'quarter') return y + '-Q' + (Math.floor(d.getMonth() / 3) + 1)
        if (g === 'year') return String(y)
        return y + '-' + mm
        `,
    ) as (value: number) => string

    return {
        toolbox: {
            show: true,
            right: 12,
            top: 8,
            feature: {
                dataZoom: { yAxisIndex: 'none', title: { zoom: '区域缩放', back: '缩放还原' } },
                restore: { title: '还原' },
                dataView: {
                    title: '数据视图',
                    lang: ['数据视图', '关闭', '刷新'],
                    readOnly: true,
                    optionToContent: dataViewOptionToContent,
                },
                saveAsImage: { title: '保存图片' },
            },
        },
        tooltip: {
            formatter: (params: any) => {
                const value = params.value
                const start = new Date(value[1]).toLocaleDateString('zh-CN')
                const end = new Date(value[2]).toLocaleDateString('zh-CN')
                const projectLine = value[7] === 'project' ? '<br/>项目汇总条' : (value[4] ? '<br/>项目: ' + value[4] : '')
                const hasDuration = Number(value[11]) === 1
                const hasDetail = Number(value[12]) === 1
                const detailLine = hasDetail && value[6] ? `<br/>${String(value[13] ?? '详情')}: ${value[6]}` : ''
                return `<b>${value[3]}</b>${projectLine}<br/>开始: ${start}<br/>结束: ${end}${hasDuration ? '<br/>周期(天): ' + value[5] : ''}${detailLine}`
            },
        },
        grid: { left: 180, right: 60, top: 20, bottom: 60 },
        xAxis: {
            type: 'time',
            min: axisMin,
            max: axisMax,
            minInterval: minIntervalFor(granularity),
            axisLabel: {
                formatter: timeLabelFormatter,
                color: axisLabelColor,
            },
            axisLine: { lineStyle: { color: axisLineColor } },
            splitLine: { show: true, lineStyle: { color: timeGridLineColor, type: 'dashed', width: 1 } },
        },
        yAxis: {
            type: 'category',
            inverse: true,
            data: taskLabels,
            axisLabel: { width: 160, overflow: 'truncate', color: axisLabelColor },
            axisLine: { lineStyle: { color: axisLineColor } },
            splitLine: { lineStyle: { color: splitLineColor } },
        },
        dataZoom: [
            { type: 'slider', xAxisIndex: 0, bottom: 10, labelFormatter: timeLabelFormatter },
            { type: 'inside', xAxisIndex: 0, zoomOnMouseWheel: false, moveOnMouseMove: false },
        ],
        series: [
            {
                name: '任务',
                type: 'custom',
                renderItem: (_params: any, api: any) => {
                    const categoryIndex = api.value(0)
                    const start = api.coord([api.value(1), categoryIndex])
                    const end = api.coord([api.value(2), categoryIndex])
                    const rowType = api.value(7)
                    const barHeight = api.size([0, 1])[1] * (rowType === 'project' ? 0.76 : 0.56)
                    const barWidth = Math.max(end[0] - start[0], 2)
                    const barColor = String(api.value(8) ?? '#5470c6')

                    const children: any[] = [{
                        type: 'rect',
                        shape: {
                            x: start[0],
                            y: start[1] - barHeight / 2,
                            width: barWidth,
                            height: barHeight,
                        },
                        style: {
                            fill: barColor,
                            opacity: rowType === 'project' ? 0.98 : 0.86,
                        },
                    }]

                    // 横道标签
                    const labelMode = String(api.value(9) ?? 'none')
                    if (labelMode !== 'none' && barWidth > 24) {
                        const taskName = String(api.value(3))
                        const durationDays = api.value(5)
                        const detail = String(api.value(6) ?? '')
                        const textColor = String(api.value(10) ?? '#ffffff')
                        const fmt = (ts: number) => new Date(ts).toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
                        let text = ''
                        if (labelMode === 'name') text = taskName
                        else if (labelMode === 'duration') text = `${durationDays}天`
                        else if (labelMode === 'dates') text = `${fmt(api.value(1))}~${fmt(api.value(2))}`
                        else if (labelMode === 'nameAndDuration') text = `${taskName} (${durationDays}天)`
                        else if (labelMode === 'detail') text = detail || taskName

                        children.push({
                            type: 'text',
                            style: {
                                text,
                                x: start[0] + 5,
                                y: start[1],
                                textAlign: 'left',
                                textVerticalAlign: 'middle',
                                fill: textColor,
                                fontSize: 11,
                                width: barWidth - 10,
                                overflow: 'truncate',
                            },
                        })
                    }

                    return { type: 'group', children }
                },
                encode: { x: [1, 2], y: 0 },
                data: barData,
            },
            ...(milestoneData.length > 0 ? [{
                type: 'scatter' as const,
                data: milestoneData,
                encode: { x: 1, y: 0 },
                tooltip: { formatter: (p: any) => `🏁 里程碑: <b>${p.name}</b>` },
            }] : []),
            {
                // 独立 Today 参考线 series（真实线段），避免 markLine 在部分环境不渲染
                name: 'TodayRef',
                type: 'line' as const,
                data: firstLabel && lastLabel
                    ? [[NOW_TS, firstLabel], [NOW_TS, lastLabel]]
                    : [],
                showSymbol: false,
                silent: true,
                tooltip: { show: false },
                lineStyle: { color: '#ef4444', type: 'dashed', width: 1.6 },
                z: 50,
                connectNulls: true,
                markLine: {
                    symbol: ['none', 'none'],
                    silent: true,
                    label: {
                        show: true,
                        formatter: 'Today',
                        position: 'insideStartTop',
                        color: '#ef4444',
                        fontSize: 11,
                    },
                    lineStyle: { color: '#ef4444', type: 'dashed', width: 1.6 },
                    data: [{ xAxis: NOW_TS }],
                },
            },
        ],
    }
})
</script>

<template>
    <BiChart :option="chartOption" :loading="props.loading" :height="props.height" />
</template>
