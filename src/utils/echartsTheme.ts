// src/utils/echartsTheme.ts
// ECharts 主题配置 (Theme Profiles)
//
// 提供主题 profile 数据、选项列表及工具函数，
// 与 vue-form 中的 echartsTheme.ts 保持一致。

export interface EChartsThemeProfile {
    name: string
    palette: string[]
    backgroundColor: string
    titleColor: string
    subtitleColor: string
    textColor: string
    axisLineColor: string
    axisLabelColor: string
    splitLineColor: string
    toolboxColor: string
    toolboxEmphasisColor: string
    tooltipAxisColor: string
    isDark: boolean
}

export interface EChartsThemeOption {
    label: string
    value: string
}

const PROFILES: Record<string, Partial<EChartsThemeProfile>> = {
    default: {
        palette: ['#c23531', '#2f4554', '#61a0a8', '#d48265', '#91c7ae', '#749f83', '#ca8622', '#bda29a', '#6e7074', '#546570', '#c4ccd3'],
        backgroundColor: 'rgba(0,0,0,0)',
        titleColor: '#333333',
        subtitleColor: '#aaaaaa',
        textColor: '#333333',
        axisLineColor: '#333333',
        axisLabelColor: '#333333',
        splitLineColor: '#cccccc',
        toolboxColor: '#999999',
        toolboxEmphasisColor: '#666666',
        tooltipAxisColor: '#cccccc',
        isDark: false,
    },
    v5: {
        palette: ['#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc'],
        backgroundColor: 'rgba(0,0,0,0)',
        titleColor: '#464646',
        subtitleColor: '#6E7079',
        textColor: '#333333',
        axisLineColor: '#6E7079',
        axisLabelColor: '#6E7079',
        splitLineColor: '#E0E6F1',
        toolboxColor: '#999999',
        toolboxEmphasisColor: '#666666',
        tooltipAxisColor: '#cccccc',
        isDark: false,
    },
    dark: {
        palette: ['#dd6b66', '#759aa0', '#e69d87', '#8dc1a9', '#ea7e53', '#eedd78', '#73a373', '#73b9bc', '#7289ab', '#91ca8c', '#f49f42'],
        backgroundColor: 'rgba(51,51,51,1)',
        titleColor: '#eeeeee',
        subtitleColor: '#aaaaaa',
        textColor: '#eeeeee',
        axisLineColor: '#eeeeee',
        axisLabelColor: '#eeeeee',
        splitLineColor: '#aaaaaa',
        toolboxColor: '#999999',
        toolboxEmphasisColor: '#666666',
        tooltipAxisColor: '#eeeeee',
        isDark: true,
    },
    vintage: { palette: ['#d87c7c', '#919e8b', '#d7ab82', '#6e7074', '#61a0a8', '#efa18d', '#787464', '#cc7e63', '#724e58', '#4b565b'], backgroundColor: '#fef8ef' },
    westeros: { palette: ['#516b91', '#59c4e6', '#edafda', '#93b7e3', '#a5e7f0', '#cbb0e3'], backgroundColor: 'transparent' },
    essos: { palette: ['#893448', '#d95850', '#eb8146', '#ffb248', '#f2d643', '#ebdba4'], backgroundColor: 'rgba(242,234,191,0.15)' },
    wonderland: { palette: ['#4ea397', '#22c3aa', '#7bd9a5', '#d0648a', '#f58db2', '#f2b3c9'], backgroundColor: 'transparent' },
    walden: { palette: ['#3fb1e3', '#6be6c1', '#626c91', '#a0a7e6', '#c4ebad', '#96dee8'], backgroundColor: 'rgba(252,252,252,0)' },
    chalk: { palette: ['#fc97af', '#87f7cf', '#f7f494', '#72ccff', '#f7c5a0', '#d4a4eb', '#d2f5a6', '#76f2f2'], backgroundColor: '#293441', isDark: true },
    infographic: {
        palette: ['#c1232b', '#27727b', '#fcce10', '#e87c25', '#b5c334', '#fe8463', '#9bca63', '#fad860', '#f3a43b', '#60c0dd', '#d7504b', '#c6e579', '#f4e001', '#f0805a', '#26c0c0'],
        backgroundColor: 'rgba(0,0,0,0)',
        titleColor: '#27727b',
        toolboxColor: '#c1232b',
        toolboxEmphasisColor: '#e87c25',
    },
    macarons: {
        palette: ['#2ec7c9', '#b6a2de', '#5ab1ef', '#ffb980', '#d87a80', '#8d98b3', '#e5cf0d', '#97b552', '#95706d', '#dc69aa', '#07a2a4', '#9a7fd1', '#588dd5', '#f5994e', '#c05050', '#59678c', '#c9ab00', '#7eb00a', '#6f5553', '#c14089'],
        backgroundColor: 'rgba(0,0,0,0)',
        titleColor: '#008acd',
        toolboxColor: '#2ec7c9',
        toolboxEmphasisColor: '#18a4a6',
    },
    roma: { palette: ['#e01f54', '#001852', '#f5e8c8', '#b8d2c7', '#c6b38e', '#a4d8c2', '#f3d999', '#d3758f', '#dcc392', '#2e4783', '#82b6e9', '#ff6347', '#a092f1', '#0a915d', '#eaf889', '#6699FF', '#ff6666', '#3cb371', '#d5b158', '#38b6b6'], backgroundColor: 'rgba(0,0,0,0)' },
    shine: { palette: ['#c12e34', '#e6b600', '#0098d9', '#2b821d', '#005eaa', '#339ca8', '#cda819', '#32a487'], backgroundColor: 'transparent' },
    'purple-passion': { palette: ['#8a7ca8', '#e098c7', '#8fd3e8', '#71669e', '#cc70af', '#7cb4cc'], backgroundColor: 'rgba(91,92,110,1)', isDark: true },
    halloween: {
        palette: ['#ff715e', '#ffaf51', '#ffee51', '#8c6ac4', '#715c87'],
        backgroundColor: 'rgba(64,64,64,0.5)',
        titleColor: '#ffaf51',
        subtitleColor: '#eeeeee',
        axisLineColor: '#666666',
        axisLabelColor: '#999999',
        splitLineColor: '#555555',
        toolboxColor: '#999999',
        toolboxEmphasisColor: '#666666',
        tooltipAxisColor: '#cccccc',
        isDark: true,
    },
}

const RUNTIME_REGISTERED_THEMES = new Set(['dark', 'vintage', 'macarons', 'shine', 'roma', 'infographic'])

export const ECHARTS_THEME_OPTIONS: EChartsThemeOption[] = [
    { label: '默认主题', value: 'default' },
    { label: 'v5', value: 'v5' },
    { label: 'Dark', value: 'dark' },
    { label: 'Vintage', value: 'vintage' },
    { label: 'Westeros', value: 'westeros' },
    { label: 'Essos', value: 'essos' },
    { label: 'Wonderland', value: 'wonderland' },
    { label: 'Walden', value: 'walden' },
    { label: 'Chalk', value: 'chalk' },
    { label: 'Infographic', value: 'infographic' },
    { label: 'Macarons', value: 'macarons' },
    { label: 'Roma', value: 'roma' },
    { label: 'Shine', value: 'shine' },
    { label: 'Purple Passion', value: 'purple-passion' },
    { label: 'Halloween', value: 'halloween' },
]

export function normalizeThemeName(themeName: string | null | undefined): string {
    const normalized = String(themeName ?? 'default').trim().toLowerCase()
    return PROFILES[normalized] ? normalized : 'default'
}

export function getThemeProfile(themeName: string | null | undefined): EChartsThemeProfile {
    const normalized = normalizeThemeName(themeName)
    const defaults: Partial<EChartsThemeProfile> = PROFILES.default ?? {}
    const chosen: Partial<EChartsThemeProfile> = PROFILES[normalized] ?? defaults

    return {
        name: normalized,
        palette: chosen.palette ?? defaults.palette ?? [],
        backgroundColor: chosen.backgroundColor ?? defaults.backgroundColor ?? 'rgba(0,0,0,0)',
        titleColor: chosen.titleColor ?? defaults.titleColor ?? '#333333',
        subtitleColor: chosen.subtitleColor ?? defaults.subtitleColor ?? '#aaaaaa',
        textColor: chosen.textColor ?? defaults.textColor ?? '#333333',
        axisLineColor: chosen.axisLineColor ?? defaults.axisLineColor ?? '#333333',
        axisLabelColor: chosen.axisLabelColor ?? defaults.axisLabelColor ?? '#333333',
        splitLineColor: chosen.splitLineColor ?? defaults.splitLineColor ?? '#cccccc',
        toolboxColor: chosen.toolboxColor ?? defaults.toolboxColor ?? '#999999',
        toolboxEmphasisColor: chosen.toolboxEmphasisColor ?? defaults.toolboxEmphasisColor ?? '#666666',
        tooltipAxisColor: chosen.tooltipAxisColor ?? defaults.tooltipAxisColor ?? '#cccccc',
        isDark: !!chosen.isDark,
    }
}

export function getEchartsRuntimeThemeName(themeName: string | null | undefined): string | undefined {
    const normalized = normalizeThemeName(themeName)
    if (normalized === 'default') return undefined
    return RUNTIME_REGISTERED_THEMES.has(normalized) ? normalized : undefined
}

// ─── 工具：将主题 profile 注入到 ECharts option ───────────────────────────────
// 对 title / legend / xAxis / yAxis / tooltip / textStyle 全量着色

function _applyAxis(axis: any, profile: EChartsThemeProfile): any {
    if (!axis) return axis
    if (Array.isArray(axis)) return axis.map(a => _applyAxis(a, profile))
    return {
        ...axis,
        axisLine: {
            ...(axis.axisLine ?? {}),
            lineStyle: {
                color: axis.axisLine?.lineStyle?.color ?? profile.axisLineColor,
                ...(axis.axisLine?.lineStyle ?? {}),
            },
        },
        axisLabel: {
            color: axis.axisLabel?.color ?? profile.axisLabelColor,
            ...(axis.axisLabel ?? {}),
        },
        splitLine: {
            ...(axis.splitLine ?? {}),
            lineStyle: {
                color: axis.splitLine?.lineStyle?.color ?? profile.splitLineColor,
                ...(axis.splitLine?.lineStyle ?? {}),
            },
        },
    }
}

function _applyLegend(legend: any, profile: EChartsThemeProfile): any {
    if (!legend) return legend
    if (Array.isArray(legend)) return legend.map(l => _applyLegend(l, profile))
    return {
        ...legend,
        textStyle: {
            color: legend.textStyle?.color ?? profile.textColor,
            ...(legend.textStyle ?? {}),
        },
    }
}

function _applyTitle(title: any, profile: EChartsThemeProfile): any {
    if (!title) return title
    if (Array.isArray(title)) return title.map(t => _applyTitle(t, profile))
    return {
        ...title,
        textStyle: {
            color: title.textStyle?.color ?? profile.titleColor,
            ...(title.textStyle ?? {}),
        },
        subtextStyle: {
            color: title.subtextStyle?.color ?? profile.subtitleColor,
            ...(title.subtextStyle ?? {}),
        },
    }
}

/**
 * 将主题 profile 完整注入到 ECharts option 对象。
 * option 自身已显式设置的字段优先，不会被覆盖。
 */
export function applyThemeProfile(option: Record<string, any>, themeName: string | null | undefined): Record<string, any> {
    const profile = getThemeProfile(themeName)
    const darkSurface = profile.isDark ? 'rgba(17,24,39,0.94)' : 'rgba(255,255,255,0.96)'

    // 判断 option 是否有实际颜色设置（透明/未设置均视为无效，使用主题色）
    const hasExplicitBg = option.backgroundColor &&
        option.backgroundColor !== 'transparent' &&
        option.backgroundColor !== 'rgba(0,0,0,0)'

    return {
        ...option,
        color: option.color ?? profile.palette,
        backgroundColor: hasExplicitBg ? option.backgroundColor : profile.backgroundColor,
        textStyle: {
            color: option.textStyle?.color ?? profile.textColor,
            ...(option.textStyle ?? {}),
        },
        title: _applyTitle(option.title, profile),
        legend: _applyLegend(option.legend, profile),
        xAxis: _applyAxis(option.xAxis, profile),
        yAxis: _applyAxis(option.yAxis, profile),
        tooltip: option.tooltip
            ? {
                  ...option.tooltip,
                  backgroundColor: option.tooltip.backgroundColor ?? darkSurface,
                  borderColor: option.tooltip.borderColor ?? profile.splitLineColor,
                  textStyle: {
                      color: option.tooltip.textStyle?.color ?? profile.textColor,
                      ...(option.tooltip.textStyle ?? {}),
                  },
              }
            : option.tooltip,
    }
}
