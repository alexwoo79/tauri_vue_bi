/**
 * AI 会话状态管理（Pinia）
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AiSession, AiMessage, AiMessageType, AiDataSource } from '../utils/aiTypes'

const STORAGE_KEY_SESSIONS = 'bi.ai.sessions.v1'
const STORAGE_KEY_CURRENT = 'bi.ai.current_session.v1'

export const useSessionStore = defineStore('aiSession', () => {
  // ────────────────────────────────────────────────────────────
  // 状态
  // ────────────────────────────────────────────────────────────

  const sessions = ref<AiSession[]>([])
  const currentSessionId = ref<string>('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 当前会话的数据源
  const currentDataSource = ref<AiDataSource | null>(null)

  // ────────────────────────────────────────────────────────────
  // 计算属性
  // ────────────────────────────────────────────────────────────

  const currentSession = computed(() => {
    return sessions.value.find((s) => s.sessionId === currentSessionId.value) || null
  })

  const currentMessages = computed(() => {
    return currentSession.value?.messages || []
  })

  const sessionList = computed(() => {
    // 按更新时间降序排列
    return [...sessions.value].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  // ────────────────────────────────────────────────────────────
  // 方法：初始化
  // ────────────────────────────────────────────────────────────

  function loadFromStorage() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY_SESSIONS)
      if (raw) {
        sessions.value = JSON.parse(raw)
      }
      const current = localStorage.getItem(STORAGE_KEY_CURRENT)
      if (current) {
        currentSessionId.value = current
      }
    } catch (e) {
      console.error('Failed to load sessions from storage:', e)
      sessions.value = []
      currentSessionId.value = ''
    }
  }

  function saveToStorage() {
    try {
      localStorage.setItem(STORAGE_KEY_SESSIONS, JSON.stringify(sessions.value))
      localStorage.setItem(STORAGE_KEY_CURRENT, currentSessionId.value)
    } catch (e) {
      console.error('Failed to save sessions to storage:', e)
    }
  }

  // ────────────────────────────────────────────────────────────
  // 方法：会话管理
  // ────────────────────────────────────────────────────────────

  function createSession(title?: string): AiSession {
    const now = Date.now()
    const sessionId = `session_${now}_${Math.random().toString(36).substring(7)}`

    const newSession: AiSession = {
      sessionId,
      title: title || `对话 ${sessions.value.length + 1}`,
      createdAt: now,
      updatedAt: now,
      messages: [],
    }

    sessions.value.push(newSession)
    currentSessionId.value = sessionId
    saveToStorage()

    return newSession
  }

  function deleteSession(sessionId: string) {
    const idx = sessions.value.findIndex((s) => s.sessionId === sessionId)
    if (idx >= 0) {
      sessions.value.splice(idx, 1)
      if (currentSessionId.value === sessionId) {
        // 切换到另一个会话，或创建新的
        currentSessionId.value = sessions.value[0]?.sessionId || ''
      }
      saveToStorage()
    }
  }

  function switchSession(sessionId: string) {
    if (sessions.value.some((s) => s.sessionId === sessionId)) {
      currentSessionId.value = sessionId
      saveToStorage()
    }
  }

  function renameSession(sessionId: string, newTitle: string) {
    const session = sessions.value.find((s) => s.sessionId === sessionId)
    if (session) {
      session.title = newTitle
      session.updatedAt = Date.now()
      saveToStorage()
    }
  }

  function clearSessionHistory(sessionId?: string) {
    const id = sessionId || currentSessionId.value
    const session = sessions.value.find((s) => s.sessionId === id)
    if (session) {
      session.messages = []
      session.updatedAt = Date.now()
      saveToStorage()
    }
  }

  // ────────────────────────────────────────────────────────────
  // 方法：消息管理
  // ────────────────────────────────────────────────────────────

  function addMessage(
    role: 'user' | 'assistant' | 'system',
    content: string,
    type: AiMessageType = 'text',
    metadata?: any
  ): AiMessage {
    if (!currentSession.value) {
      throw new Error('No active session')
    }

    const now = Date.now()
    const message: AiMessage = {
      id: `msg_${now}_${Math.random().toString(36).substring(7)}`,
      role,
      content,
      timestamp: now,
      type,
      metadata,
    }

    currentSession.value.messages.push(message)
    currentSession.value.updatedAt = now
    saveToStorage()

    return message
  }

  function updateMessage(messageId: string, updates: Partial<AiMessage>) {
    if (!currentSession.value) return

    const msg = currentSession.value.messages.find((m) => m.id === messageId)
    if (msg) {
      Object.assign(msg, updates)
      currentSession.value.updatedAt = Date.now()
      saveToStorage()
    }
  }

  function appendToMessage(messageId: string, content: string) {
    if (!currentSession.value) return

    const msg = currentSession.value.messages.find((m) => m.id === messageId)
    if (msg) {
      msg.content += content
      currentSession.value.updatedAt = Date.now()
      saveToStorage()
    }
  }

  function deleteMessage(messageId: string) {
    if (!currentSession.value) return

    const idx = currentSession.value.messages.findIndex((m) => m.id === messageId)
    if (idx >= 0) {
      currentSession.value.messages.splice(idx, 1)
      currentSession.value.updatedAt = Date.now()
      saveToStorage()
    }
  }

  // ────────────────────────────────────────────────────────────
  // 方法：状态管理
  // ────────────────────────────────────────────────────────────

  function setLoading(loading: boolean) {
    isLoading.value = loading
  }

  function setError(err: string | null) {
    error.value = err
  }

  function setDataSource(source: AiDataSource | null) {
    currentDataSource.value = source
    if (currentSession.value) {
      currentSession.value.dataSourceId = source?.id
      saveToStorage()
    }
  }

  // ────────────────────────────────────────────────────────────
  // 方法：模型配置
  // ────────────────────────────────────────────────────────────

  function setSelectedModel(sessionId: string, modelId: string) {
    const session = sessions.value.find((s) => s.sessionId === sessionId)
    if (session) {
      session.selectedModelId = modelId
      saveToStorage()
    }
  }

  function getSelectedModel(sessionId?: string): string | undefined {
    const id = sessionId || currentSessionId.value
    const session = sessions.value.find((s) => s.sessionId === id)
    return session?.selectedModelId
  }

  return {
    // 状态
    sessions,
    currentSessionId,
    isLoading,
    error,
    currentDataSource,

    // 计算属性
    currentSession,
    currentMessages,
    sessionList,

    // 方法
    loadFromStorage,
    saveToStorage,
    createSession,
    deleteSession,
    switchSession,
    renameSession,
    clearSessionHistory,
    addMessage,
    updateMessage,
    appendToMessage,
    deleteMessage,
    setLoading,
    setError,
    setDataSource,
    setSelectedModel,
    getSelectedModel,
  }
})
