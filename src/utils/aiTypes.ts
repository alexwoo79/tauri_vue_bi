// src/utils/aiTypes.ts
// AI integration contract types for frontend.

import type { ChartType } from './chartTypes'

export interface AiChartParams {
    chartType: ChartType
    xCol: string
    yCols: string[]
    colorCol?: string | null
    sortBy: 'x' | 'y' | 'none'
    sortAsc: boolean
    topN: number
}

export interface AiSuggestChartResult {
    recommendation: AiChartParams
    reasoning: string
    confidence: number
    warnings: string[]
}

export interface ApiResult<T> {
    ok: boolean
    data?: T
    error?: string
}
