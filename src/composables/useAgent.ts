// src/composables/useAgent.ts
//
// Agent Composable - 封装 Tauri Rust Agent 的调用
//
// 提供以下功能：
// - 会话管理（创建、删除、列出）
// - 流式聊天（SSE 事件监听）
// - 图表生成
// - 文件导出（Excel/PPT/Report/Dashboard）

import { ref, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import type { AiEventType } from '@/utils/aiTypes'

export interface SseEvent {
  type: AiEventType
  content?: string
  message?: string
  tool?: string
  tool_name?: string
  display?: string
  html?: string
  chart_type?: string
  chartId?: string
  chart_id?: string
  echarts_spec?: Record<string, any>
  meta?: Record<string, any>
  file_path?: string
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

export function useAgent() {
  const isProcessing = ref(false)
  const eventListeners = ref<UnlistenFn[]>([])

  /**
   * 创建新会话
   */
  async function createSession(modelId: string): Promise<string> {
    try {
      const sessionId = await invoke<string>('create_session', {
        modelId,
      })
      return sessionId
    } catch (error) {
      console.error('Failed to create session:', error)
      throw error
    }
  }

  /**
   * 删除会话
   */
  async function deleteSession(sessionId: string): Promise<void> {
    try {
      await invoke('delete_session', { sessionId })
    } catch (error) {
      console.error('Failed to delete session:', error)
      throw error
    }
  }

  /**
   * 列出所有会话
   */
  async function listSessions(): Promise<Array<{ id: string; title: string; created_at: number }>> {
    try {
      const sessions = await invoke<Array<{ id: string; title: string; created_at: number }>>('list_sessions')
      return sessions
    } catch (error) {
      console.error('Failed to list sessions:', error)
      throw error
    }
  }

  /**
   * 清除会话历史
   */
  async function clearHistory(sessionId: string): Promise<void> {
    try {
      await invoke('clear_session_history', { sessionId })
    } catch (error) {
      console.error('Failed to clear history:', error)
      throw error
    }
  }

  /**
   * 停止会话
   */
  async function stopSession(sessionId: string): Promise<void> {
    try {
      await invoke('stop_session', { sessionId })
    } catch (error) {
      console.error('Failed to stop session:', error)
      throw error
    }
  }

  /**
   * 流式聊天
   * @param sessionId 会话 ID
   * @param userMessage 用户消息
   * @param callback SSE 事件回调
   */
  async function chatStream(
    sessionId: string,
    userMessage: string,
    callback: (event: SseEvent) => void,
    options?: {
      command?: string
      provider?: string
      model?: string
      apiKey?: string
      baseUrl?: string
    }
  ): Promise<void> {
    isProcessing.value = true

    try {
      // 保证同一时刻只保留一个 SSE 监听器，避免重复回调。
      await cleanup()

      // 订阅 SSE 事件
      const unlisten = await listen<SseEvent>('sse-event', (event) => {
        callback(event.payload)
      })
      eventListeners.value.push(unlisten)

      // 调用聊天命令
      await invoke('chat_stream', {
        sessionId,
        userMessage,
        command: options?.command,
        provider: options?.provider,
        model: options?.model,
        apiKey: options?.apiKey,
        baseUrl: options?.baseUrl,
      })
    } catch (error) {
      console.error('Chat stream error:', error)
      callback({
        type: 'error',
        error: String(error),
      })
    } finally {
      isProcessing.value = false
    }
  }

  /**
   * 生成图表
   */
  async function generateChart(
    chartType: string,
    data: Array<Record<string, any>>,
    mapping: Record<string, any>,
    options?: Record<string, any>
  ): Promise<{ html: string; chart_type: string; warnings: string[]; meta: Record<string, any> }> {
    try {
      const result = await invoke<{ html: string; chart_type: string; warnings: string[]; meta: Record<string, any> }>('generate_chart', {
        chartType,
        data,
        mapping,
        options,
      })
      return result
    } catch (error) {
      console.error('Generate chart error:', error)
      throw error
    }
  }

  async function chartWorkflow(
    chartType: string,
    xCol: string,
    yCol: string,
    callback: (event: SseEvent) => void,
    title?: string
  ): Promise<void> {
    isProcessing.value = true

    try {
      await cleanup()

      const unlisten = await listen<SseEvent>('sse-event', (event) => {
        callback(event.payload)
      })
      eventListeners.value.push(unlisten)

      await invoke('chart_workflow', {
        chartType,
        xCol,
        yCol,
        title,
      })
    } catch (error) {
      console.error('Chart workflow error:', error)
      callback({
        type: 'error',
        error: String(error),
      })
    } finally {
      isProcessing.value = false
    }
  }

  /**
   * 导出 Excel
   */
  async function exportExcel(
    tables: string[],
    filename?: string
  ): Promise<{ file_path: string; message: string }> {
    try {
      const result = await invoke<{ file_path: string; message: string }>('export_excel', {
        tables,
        filename,
      })
      return result
    } catch (error) {
      console.error('Export Excel error:', error)
      throw error
    }
  }

  /**
   * 生成 PPT
   */
  async function generatePPT(
    title: string,
    colorScheme: string = 'mckinsey',
    slideCount: number = 5
  ): Promise<{ file_path: string; message: string }> {
    try {
      const result = await invoke<{ file_path: string; message: string }>('generate_ppt', {
        title,
        colorScheme,
        slideCount,
      })
      return result
    } catch (error) {
      console.error('Generate PPT error:', error)
      throw error
    }
  }

  /**
   * 生成报告
   */
  async function generateReport(
    title: string,
    sectionCount: number = 3
  ): Promise<{ file_path: string; message: string }> {
    try {
      const result = await invoke<{ file_path: string; message: string }>('generate_report', {
        title,
        sectionCount,
      })
      return result
    } catch (error) {
      console.error('Generate report error:', error)
      throw error
    }
  }

  /**
   * 生成 Dashboard
   */
  async function generateDashboard(
    name: string,
    colorScheme: string = 'mckinsey',
    widgetCount: number = 6
  ): Promise<{ file_path: string; message: string }> {
    try {
      const result = await invoke<{ file_path: string; message: string }>('generate_dashboard', {
        name,
        colorScheme,
        widgetCount,
      })
      return result
    } catch (error) {
      console.error('Generate dashboard error:', error)
      throw error
    }
  }

  /**
   * 清理事件监听器
   */
  async function cleanup() {
    for (const unlisten of eventListeners.value) {
      unlisten()
    }
    eventListeners.value = []
  }

  // 组件卸载时自动清理
  onUnmounted(() => {
    cleanup()
  })

  return {
    // 状态
    isProcessing,

    // 会话管理
    createSession,
    deleteSession,
    listSessions,
    clearHistory,
    stopSession,

    // 聊天
    chatStream,
    generateChart,
    chartWorkflow,

    // 导出
    exportExcel,
    generatePPT,
    generateReport,
    generateDashboard,

    // 清理
    cleanup,
  }
}
