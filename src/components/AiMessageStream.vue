/**
 * AI 消息流显示组件
 * 显示对话消息、处理消息格式化
 */

<template>
  <div ref="streamRef" class="message-stream" @scroll.passive="handleStreamScroll">
    <!-- 消息列表 -->
    <div class="messages-container" @click.capture="handleMessageClick">
      <div
        v-for="msg in messages"
        :key="msg.id"
        :class="['message-group', `msg-${msg.role}`]"
      >
        <!-- 用户消息 -->
        <div v-if="msg.role === 'user'" class="message-bubble user">
          <div class="message-content">{{ msg.content }}</div>
        </div>

        <!-- AI 消息 -->
        <div v-else-if="msg.role === 'assistant'" :class="['message-bubble', 'assistant', { 'chart-bubble': msg.type === 'chart_html' }]">
          <div v-if="msg.type === 'text_delta'" class="message-content">
            <div v-html="renderMarkdown(msg.content)"></div>
          </div>
          <div v-else-if="msg.type === 'tool_start'" class="tool-start">
            <el-icon class="spin"><Loading /></el-icon>
            <span>{{ msg.metadata?.toolName || '工具调用' }}</span>
            <span class="tool-hint">{{ msg.metadata?.display }}</span>
          </div>
          <div v-else-if="msg.type === 'tool_result'" class="tool-result">
            <el-icon class="success"><CircleCheck /></el-icon>
            <span>{{ msg.metadata?.display || '工具执行完成' }}</span>
          </div>
          <div v-else-if="msg.type === 'chart_html'" class="chart-wrapper">
            <button class="chart-expand-btn" @click="openChartFullscreen(msg.content)">全屏查看</button>
            <iframe
              :srcdoc="msg.content"
              class="chart-iframe"
              frameborder="0"
              scrolling="auto"
            />
          </div>
          <div v-else-if="msg.type === 'chart_generated'" class="chart-wrapper">
            <button class="chart-expand-btn" @click="openChartFullscreen(msg.html || msg.content)">全屏查看</button>
            <iframe
              :srcdoc="msg.html || msg.content"
              class="chart-iframe"
              frameborder="0"
              scrolling="auto"
            />
            <div v-if="msg.chartType" class="chart-meta">
              <el-tag size="small">{{ msg.chartType }}</el-tag>
              <span v-if="msg.meta?.n_rows" class="meta-info">{{ msg.meta.n_rows }} 行数据</span>
            </div>
          </div>
          <div v-else-if="msg.type === 'code_block'" class="code-block">
            <pre><code v-html="highlightCode(msg.content)"></code></pre>
          </div>
          <div v-else-if="msg.type === 'error'" class="error-message">
            <el-icon><Warning /></el-icon>
            <span>{{ msg.metadata?.error || msg.content }}</span>
          </div>
          <div v-else-if="msg.type === 'thinking'" class="thinking-note">{{ msg.content }}</div>
          <div v-else-if="msg.type === 'outline'" class="outline-card">
            <div class="message-content" v-html="renderMarkdown(msg.content)"></div>
            <div class="outline-actions">
              <el-button size="small" type="primary" @click="emitOutlineAction(msg, 'confirm')">确认</el-button>
              <el-button size="small" @click="emitOutlineAction(msg, 'revise')">修改</el-button>
              <el-button size="small" text @click="emitOutlineAction(msg, 'cancel')">取消</el-button>
            </div>
          </div>
          <div v-else class="message-content">
            <div v-html="renderMarkdown(msg.content)"></div>
          </div>
        </div>

        <!-- 系统消息 -->
        <div v-else class="system-message">
          <div v-html="renderMarkdown(msg.content)"></div>
        </div>
      </div>

      <!-- 加载中提示 -->
      <div v-if="isStreaming" class="message-group msg-assistant">
        <div class="message-bubble assistant">
          <div class="loading-dots">
            <span></span>
            <span></span>
            <span></span>
          </div>
        </div>
      </div>
    </div>

    <el-dialog v-model="chartFullscreenVisible" width="96vw" top="2vh" class="chart-fullscreen-dialog" append-to-body>
      <iframe
        v-if="fullscreenChartHtml"
        :srcdoc="fullscreenChartHtml"
        class="chart-fullscreen-iframe"
        frameborder="0"
        scrolling="auto"
      />
    </el-dialog>

    <el-dialog v-model="dashboardPreviewVisible" width="96vw" top="2vh" class="dashboard-preview-dialog" append-to-body>
      <div v-if="dashboardPreviewLoading" class="dashboard-preview-loading">看板加载中...</div>
      <div v-else-if="dashboardPreviewError" class="dashboard-preview-error">
        <div>看板打开失败：{{ dashboardPreviewError }}</div>
        <div class="dashboard-preview-url">URL: {{ dashboardPreviewUrl }}</div>
      </div>
      <iframe
        v-else-if="dashboardPreviewUrl"
        :src="dashboardPreviewUrl"
        class="dashboard-preview-iframe"
        frameborder="0"
        scrolling="auto"
        @load="handleDashboardLoaded"
      />
    </el-dialog>

    <button
      v-if="!autoFollowProgress"
      type="button"
      class="jump-latest-btn"
      @click="jumpToLatest"
    >
      查看最新进度<span v-if="pendingUpdates > 0"> ({{ pendingUpdates }})</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { Loading, Warning, CircleCheck } from '@element-plus/icons-vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import type { AiMessage } from '../utils/aiTypes'

interface Props {
  messages: AiMessage[]
  isStreaming?: boolean
  apiBaseUrl?: string
  apiToken?: string
}

interface Emits {
  (e: 'outline-action', payload: {
    action: 'confirm' | 'revise' | 'cancel'
    outlineType: 'excel' | 'report' | 'ppt' | 'dashboard'
    tables?: string[]
    filename?: string
    title?: string
    sections?: Record<string, any>[]
    slides?: Record<string, any>[]
    name?: string
    widgets?: Record<string, any>[]
  }): void
}

const props = withDefaults(defineProps<Props>(), {
  isStreaming: false,
  apiBaseUrl: '',
  apiToken: '',
})
const emit = defineEmits<Emits>()

const streamRef = ref<HTMLDivElement>()
const chartFullscreenVisible = ref(false)
const fullscreenChartHtml = ref('')
const dashboardPreviewVisible = ref(false)
const dashboardPreviewUrl = ref('')
const dashboardPreviewLoading = ref(false)
const dashboardPreviewError = ref('')
const autoFollowProgress = ref(true)
const pendingUpdates = ref(0)
let dashboardPreviewTimer: number | null = null
const BOTTOM_THRESHOLD_PX = 96

function isNearBottom(): boolean {
  const el = streamRef.value
  if (!el) return true
  const distance = el.scrollHeight - (el.scrollTop + el.clientHeight)
  return distance <= BOTTOM_THRESHOLD_PX
}

function scrollToBottom(behavior: ScrollBehavior = 'auto') {
  const el = streamRef.value
  if (!el) return
  el.scrollTo({ top: el.scrollHeight, behavior })
}

function handleStreamScroll() {
  const nearBottom = isNearBottom()
  autoFollowProgress.value = nearBottom
  if (nearBottom) pendingUpdates.value = 0
}

function jumpToLatest() {
  autoFollowProgress.value = true
  pendingUpdates.value = 0
  nextTick(() => scrollToBottom('smooth'))
}

// 消息推进时：仅在“跟随模式”下自动贴底；否则只累计未读更新计数
watch(
  () => `${props.messages.length}|${props.messages[props.messages.length - 1]?.id || ''}|${props.messages[props.messages.length - 1]?.content?.length || 0}|${props.isStreaming}`,
  async () => {
    await nextTick()
    if (autoFollowProgress.value || isNearBottom()) {
      scrollToBottom('auto')
      pendingUpdates.value = 0
      autoFollowProgress.value = true
    } else {
      pendingUpdates.value += 1
    }
  }
)

// 代码高亮（简单实现，后续可集成 highlight.js）
function highlightCode(code: string): string {
  // 这里简单返回，实际可以使用 highlight.js 或其他库
  return code.replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

// ✅ 格式化工具结果内容（JSON 美化显示）
function formatToolResult(content: string): string {
  try {
    // 尝试解析为 JSON 并格式化
    const parsed = JSON.parse(content)
    return JSON.stringify(parsed, null, 2)
  } catch {
    // 如果不是 JSON，直接返回原始内容
    return content
  }
}

function renderMarkdown(content: string): string {
  if (!content) return ''
  const rawHtml = marked.parse(content, { breaks: true }) as string
  const clean = DOMPurify.sanitize(rawHtml)

  // Rewrite relative backend links (e.g. /api/export/...) to absolute Flask URLs
  // so downloads work correctly from the Tauri/Vite webview origin.
  const parser = new DOMParser()
  const doc = parser.parseFromString(clean, 'text/html')
  const anchors = doc.querySelectorAll('a[href]')
  anchors.forEach((a) => {
    const href = a.getAttribute('href') || ''
    const absHref = href.startsWith('/api/') || href.startsWith('/dashboard/')
      ? `${props.apiBaseUrl}${href}`
      : href

    if (href.includes('/api/export/')) {
      const btn = doc.createElement('button')
      btn.setAttribute('type', 'button')
      btn.className = 'inline-link-btn export'
      btn.setAttribute('data-export-url', absHref)
      btn.textContent = a.textContent || '下载文件'
      a.replaceWith(btn)
      return
    }

    if (href.includes('/dashboard/')) {
      const btn = doc.createElement('button')
      btn.setAttribute('type', 'button')
      btn.className = 'inline-link-btn dashboard'
      btn.setAttribute('data-dashboard-url', absHref)
      btn.textContent = a.textContent || '打开看板'
      a.replaceWith(btn)
      return
    }

    a.setAttribute('href', absHref)
    a.setAttribute('target', '_blank')
    a.setAttribute('rel', 'noopener noreferrer')
  })
  return doc.body.innerHTML
}

function openChartFullscreen(html: string) {
  fullscreenChartHtml.value = html
  chartFullscreenVisible.value = true
}

function handleDashboardLoaded() {
  dashboardPreviewLoading.value = false
  dashboardPreviewError.value = ''
  if (dashboardPreviewTimer !== null) {
    window.clearTimeout(dashboardPreviewTimer)
    dashboardPreviewTimer = null
  }
}

function startDashboardPreview(url: string) {
  dashboardPreviewUrl.value = url
  dashboardPreviewError.value = ''
  dashboardPreviewLoading.value = true
  dashboardPreviewVisible.value = true

  if (dashboardPreviewTimer !== null) {
    window.clearTimeout(dashboardPreviewTimer)
    dashboardPreviewTimer = null
  }
  dashboardPreviewTimer = window.setTimeout(() => {
    if (dashboardPreviewLoading.value) {
      dashboardPreviewLoading.value = false
      dashboardPreviewError.value = '加载超时，请确认 Python Flask 服务可用且 dashboard 路由可访问。'
    }
  }, 10000)
}

watch(
  () => dashboardPreviewVisible.value,
  (visible) => {
    if (!visible) {
      if (dashboardPreviewTimer !== null) {
        window.clearTimeout(dashboardPreviewTimer)
        dashboardPreviewTimer = null
      }
      dashboardPreviewLoading.value = false
      dashboardPreviewError.value = ''
      dashboardPreviewUrl.value = ''
    }
  }
)

async function handleMessageClick(e: MouseEvent) {
  const rawTarget = e.target
  const baseEl = rawTarget instanceof Element
    ? rawTarget
    : rawTarget instanceof Node
      ? rawTarget.parentElement
      : null
  const exportBtn = baseEl?.closest('button[data-export-url]') as HTMLButtonElement | null
  const dashboardBtn = baseEl?.closest('button[data-dashboard-url]') as HTMLButtonElement | null

  let url = ''
  let isExportLink = false
  let isDashboardLink = false

  if (exportBtn) {
    url = exportBtn.dataset.exportUrl || ''
    isExportLink = !!url
  } else if (dashboardBtn) {
    url = dashboardBtn.dataset.dashboardUrl || ''
    isDashboardLink = !!url
  } else {
    const anchor = baseEl?.closest('a[href]') as HTMLAnchorElement | null
    if (!anchor) return
    const href = anchor.getAttribute('href') || ''
    isExportLink = href.includes('/api/export/')
    isDashboardLink = href.includes('/dashboard/')
    if (!isExportLink && !isDashboardLink) return
    url = href.startsWith('http') ? href : `${props.apiBaseUrl}${href}`
  }

  e.preventDefault()
  if (!url) return

  if (isDashboardLink) {
    startDashboardPreview(url)
    return
  }

  try {
    const headers = new Headers()
    if (props.apiToken) {
      headers.set('Authorization', `Bearer ${props.apiToken}`)
    }
    const resp = await fetch(url, { headers })
    if (!resp.ok) throw new Error(`下载失败: ${resp.status}`)
    const blob = await resp.blob()

    const filename = decodeURIComponent(url.split('/').pop() || 'export.bin')
    const blobUrl = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = blobUrl
    a.download = filename
    document.body.appendChild(a)
    a.click()
    a.remove()
    window.setTimeout(() => URL.revokeObjectURL(blobUrl), 3000)
    ElMessage.success(`开始下载：${filename}`)
  } catch (err) {
    console.error('[AiMessageStream] export download failed:', err)
    ElMessage.error('下载失败，请确认 Flask 服务与导出文件是否可用。')
  }
}

function emitOutlineAction(msg: AiMessage, action: 'confirm' | 'revise' | 'cancel') {
  const outlineType = msg.metadata?.outlineType
  if (!outlineType) return
  emit('outline-action', {
    action,
    outlineType,
    tables: msg.metadata?.tables,
    filename: msg.metadata?.filename,
    title: msg.metadata?.title,
    sections: msg.metadata?.sections,
    slides: msg.metadata?.slides,
    name: msg.metadata?.name,
    widgets: msg.metadata?.widgets,
  })
}

</script>

<style scoped>
.message-stream {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
  position: relative;
  padding: 16px;
  gap: 8px;
  background-color: var(--el-bg-color);
}

.jump-latest-btn {
  position: absolute;
  right: 16px;
  bottom: 12px;
  z-index: 20;
  border: 1px solid var(--el-color-primary-light-5);
  background: rgba(255, 255, 255, 0.95);
  color: var(--el-color-primary);
  border-radius: 999px;
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
}

.jump-latest-btn:hover {
  border-color: var(--el-color-primary);
}

.messages-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.message-group {
  display: flex;
  animation: slideIn 0.3s ease;
}

.msg-user {
  justify-content: flex-end;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message-bubble {
  max-width: 70%;
  padding: 12px 16px;
  border-radius: 8px;
  word-break: break-word;
  line-height: 1.5;

  &.user {
    background-color: var(--el-color-primary);
    color: white;
  }

  &.assistant {
    background-color: var(--el-fill-color-light);
    color: var(--el-text-color-primary);
    max-width: 94%;
    margin-left: 4px;
  }
}

.message-bubble.chart-bubble {
  width: 550px;
  max-width: 100%;
  padding: 10px 12px;
}

.message-content {
  font-size: 14px;

  :deep(ol),
  :deep(ul) {
    padding-left: 1.6em;
    margin: 4px 0;
  }

  :deep(li) {
    margin: 2px 0;
  }
}

.tool-start {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--el-text-color-regular);
}

.tool-start :deep(.el-icon) {
  font-size: 16px;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.tool-hint {
  color: var(--el-text-color-secondary);
  font-size: 11px;
  margin-left: auto;
}

/* ✅ 工具执行完成样式（对齐 Python 版本） */
.tool-result {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--el-color-success);
}

.tool-result :deep(.el-icon) {
  font-size: 16px;
}

.chart-wrapper {
  width: 100%;
  min-height: 520px;
  height: 66vh;
  margin-top: 8px;
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  overflow: hidden;
  background: white;
  position: relative;
}

.chart-iframe {
  width: 100%;
  height: 100%;
}

.chart-expand-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 8;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(4px);
  border: 1px solid var(--el-border-color);
  border-radius: 6px;
  padding: 4px 9px;
  font-size: 12px;
  color: var(--el-text-color-regular);
  cursor: pointer;
}

.chart-expand-btn:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
}

.chart-fullscreen-iframe {
  width: 100%;
  height: 88vh;
}

.chart-fullscreen-dialog :deep(.el-dialog__body) {
  padding: 8px;
}

.dashboard-preview-iframe {
  width: 100%;
  height: 88vh;
}

.dashboard-preview-loading,
.dashboard-preview-error {
  min-height: 120px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: var(--el-text-color-regular);
  font-size: 14px;
}

.dashboard-preview-url {
  margin-top: 8px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  word-break: break-all;
}

.message-content :deep(.inline-link-btn) {
  border: 1px solid var(--el-border-color);
  background: var(--el-fill-color-light);
  color: var(--el-color-primary);
  border-radius: 6px;
  padding: 4px 10px;
  cursor: pointer;
  font-size: 13px;
}

.message-content :deep(.inline-link-btn:hover) {
  border-color: var(--el-color-primary);
}

.dashboard-preview-dialog :deep(.el-dialog__body) {
  padding: 8px;
}

.code-block {
  background-color: #282c34;
  color: #abb2bf;
  padding: 12px;
  border-radius: 4px;
  margin-top: 8px;
  overflow-x: auto;

  pre {
    margin: 0;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 12px;
    line-height: 1.4;
  }
}

.error-message {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--el-color-danger);
  font-size: 12px;
}

.thinking-note {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.55;
}

.outline-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.outline-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.system-message {
  text-align: center;
  color: var(--el-text-color-secondary);
  font-size: 11px;
  padding: 5px 0;
}

.loading-dots {
  display: flex;
  gap: 4px;

  span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--el-text-color-regular);
    animation: bounce 1.4s infinite;

    &:nth-child(1) {
      animation-delay: -0.32s;
    }

    &:nth-child(2) {
      animation-delay: -0.16s;
    }
  }
}

@keyframes bounce {
  0%,
  80%,
  100% {
    opacity: 0.3;
    transform: scaleY(0.6);
  }
  40% {
    opacity: 1;
    transform: scaleY(1);
  }
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--el-border-color);
  border-radius: 3px;

  &:hover {
    background: var(--el-border-color-darker);
  }
}
</style>
