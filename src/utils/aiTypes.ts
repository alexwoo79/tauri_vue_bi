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

// ─────────────────────────────────────────────────────────────
// 会话相关类型
// ─────────────────────────────────────────────────────────────

export interface AiSession {
    sessionId: string
    title: string
    createdAt: number // 时间戳
    updatedAt: number
    messages: AiMessage[]
    selectedModelId?: string
    dataSourceId?: string
}

export interface AiMessage {
    id: string
    role: 'user' | 'assistant' | 'system'
    content: string
    timestamp: number
    type: AiMessageType
    metadata?: {
        toolName?: string
        toolInput?: Record<string, any>
        display?: string
        chartId?: string
        error?: string
        outlineType?: 'excel' | 'report' | 'ppt' | 'dashboard'
        tables?: string[]
        filename?: string
        title?: string
        sections?: Record<string, any>[]
        slides?: Record<string, any>[]
        name?: string
        widgets?: Record<string, any>[]
        inputTokens?: number
        outputTokens?: number
        sessionTotalOutput?: number
        contextWindow?: number
        maxOutputTokens?: number
    }
}

export type AiMessageType =
    | 'text'
    | 'text_delta'
    | 'outline'
    | 'tool_start'
    | 'tool_result'
    | 'chart_html'
    | 'code_block'
    | 'error'
    | 'thinking'
    | 'usage'

// ─────────────────────────────────────────────────────────────
// SSE 事件类型
// ─────────────────────────────────────────────────────────────

export interface AiEvent {
    type: AiEventType
    content?: string
    message?: string
    tool?: string
    display?: string
    html?: string
    chartId?: string
    chart_id?: string
    tables?: string[]
    filename?: string
    title?: string
    sections?: Record<string, any>[]
    slides?: Record<string, any>[]
    name?: string
    widgets?: Record<string, any>[]
    markdown?: string
    error?: string
    reasoning?: string
    usage?: {
        input_tokens: number
        output_tokens: number
        total_tokens: number
    }
}

export type AiEventType =
    | 'text'
    | 'text_delta'
    | 'tool_start'
    | 'tool_result'
    | 'excel_outline'
    | 'report_outline'
    | 'ppt_outline'
    | 'dashboard_outline'
    | 'chart_ref'
    | 'chart_html'
    | 'code_block'
    | 'thinking'
    | 'reasoning'
    | 'usage'
    | 'done'
    | 'stopped'
    | 'error'
    | 'stop'

// ─────────────────────────────────────────────────────────────
// 模型配置
// ─────────────────────────────────────────────────────────────

export interface AiModelConfig {
    id: string
    provider: string
    displayName: string
    apiKey: string
    baseUrl: string
    model: string
    enabled: boolean
    isCustom: boolean
    contextWindow: number | null
    maxOutputTokens: number | null
    enableThinking: boolean
}

// ─────────────────────────────────────────────────────────────
// Agent 工具定义
// ─────────────────────────────────────────────────────────────

export interface AiTool {
    name: string
    description: string
    category: 'data' | 'analysis' | 'export' | 'visualization'
    enabled: boolean
    parameters?: Record<string, any>
}

// ─────────────────────────────────────────────────────────────
// 数据源
// ─────────────────────────────────────────────────────────────

export interface AiDataSource {
    id: string
    name: string
    type: 'csv' | 'excel' | 'sql' | 'gsheets' | 'api'
    config: Record<string, any>
    schema?: Array<{
        name: string
        type: string
    }>
    rowCount?: number
}

// ─────────────────────────────────────────────────────────────
// 导出配置
// ─────────────────────────────────────────────────────────────

export interface ExportConfig {
    format: 'ppt' | 'excel' | 'pdf' | 'markdown'
    title?: string
    filename?: string
    includeCharts?: boolean
    includeData?: boolean
}
