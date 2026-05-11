// Business-friendly table header labels shared by analysis views.
export const BUSINESS_COLUMN_LABEL_MAP: Record<string, string> = {
    '年份': '统计年度',
    '年月': '统计月份',
    '年季': '统计季度',
    '年周': '统计周',
    '上期同比值': '去年同期值',
    '上期环比值': '上期值',
    '同比增长率%': '同比完成率%',
    '环比增长率%': '环比完成率%',
    '累计值': '累计完成值',
    '累计上期同比值': '去年同期累计值',
    '累计上期环比值': '上期累计值',
    '累计同比增长率%': '累计完成率(同比)%',
    '累计环比增长率%': '累计完成率(环比)%',
    '标准化Z分数': '标准化Z分数',
    '基期指数(100)': '基期指数(100)',
}

const DYNAMIC_SUFFIX_LABEL_MAP: Record<string, string> = {
    '累计值': '累计完成值',
    '上期同比值': '去年同期值',
    '上期环比值': '上期值',
    '同比增长率%': '同比完成率%',
    '环比增长率%': '环比完成率%',
    '累计上期同比值': '去年同期累计值',
    '累计上期环比值': '上期累计值',
    '累计同比增长率%': '累计完成率(同比)%',
    '累计环比增长率%': '累计完成率(环比)%',
    '标准化Z分数': '标准化Z分数',
    '基期指数(100)': '基期指数(100)',
}

export function getBusinessColumnLabel(columnName: string): string {
    const exact = BUSINESS_COLUMN_LABEL_MAP[columnName]
    if (exact) return exact

    const plainYearCompare = /^前(\d+)年同期值$/.exec(columnName)
    if (plainYearCompare) {
        return `${plainYearCompare[1]}年前同期值`
    }
    const plainYearGrowth = /^对前(\d+)年同比增长率%$/.exec(columnName)
    if (plainYearGrowth) {
        return `较${plainYearGrowth[1]}年前同比完成率%`
    }
    const plainCumYearCompare = /^累计前(\d+)年同期值$/.exec(columnName)
    if (plainCumYearCompare) {
        return `累计${plainCumYearCompare[1]}年前同期值`
    }
    const plainCumYearGrowth = /^累计对前(\d+)年同比增长率%$/.exec(columnName)
    if (plainCumYearGrowth) {
        return `累计较${plainCumYearGrowth[1]}年前同比完成率%`
    }

    const prefYearCompare = /^(.+)_前(\d+)年同期值$/.exec(columnName)
    if (prefYearCompare) {
        const prefix = BUSINESS_COLUMN_LABEL_MAP[prefYearCompare[1]] ?? prefYearCompare[1]
        return `${prefix}（${prefYearCompare[2]}年前同期值）`
    }
    const prefYearGrowth = /^(.+)_对前(\d+)年同比增长率%$/.exec(columnName)
    if (prefYearGrowth) {
        const prefix = BUSINESS_COLUMN_LABEL_MAP[prefYearGrowth[1]] ?? prefYearGrowth[1]
        return `${prefix}（较${prefYearGrowth[2]}年前同比完成率%）`
    }
    const prefCumYearCompare = /^(.+)_累计前(\d+)年同期值$/.exec(columnName)
    if (prefCumYearCompare) {
        const prefix = BUSINESS_COLUMN_LABEL_MAP[prefCumYearCompare[1]] ?? prefCumYearCompare[1]
        return `${prefix}（累计${prefCumYearCompare[2]}年前同期值）`
    }
    const prefCumYearGrowth = /^(.+)_累计对前(\d+)年同比增长率%$/.exec(columnName)
    if (prefCumYearGrowth) {
        const prefix = BUSINESS_COLUMN_LABEL_MAP[prefCumYearGrowth[1]] ?? prefCumYearGrowth[1]
        return `${prefix}（累计较${prefCumYearGrowth[2]}年前同比完成率%）`
    }

    for (const [suffix, mapped] of Object.entries(DYNAMIC_SUFFIX_LABEL_MAP)) {
        const marker = `_${suffix}`
        if (!columnName.endsWith(marker)) continue
        const rawPrefix = columnName.slice(0, columnName.length - marker.length)
        const prefix = BUSINESS_COLUMN_LABEL_MAP[rawPrefix] ?? rawPrefix
        return `${prefix}（${mapped}）`
    }

    return columnName
}

export function getBusinessOptionLabel(columnName: string): string {
    const mapped = getBusinessColumnLabel(columnName)
    return mapped === columnName ? columnName : `${mapped} (${columnName})`
}
