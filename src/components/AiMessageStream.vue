/**
 * AI 消息流显示组件
 * 显示对话消息、处理消息格式化
 */

<template>
  <div class="message-stream">
    <!-- 消息列表 -->
    <div class="messages-container">
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
          <div v-else-if="msg.type === 'chart_html'" class="chart-wrapper">
            <button class="chart-expand-btn" @click="openChartFullscreen(msg.content)">全屏查看</button>
            <iframe
              :srcdoc="msg.content"
              class="chart-iframe"
              frameborder="0"
              scrolling="auto"
            />
          </div>
          <div v-else-if="msg.type === 'code_block'" class="code-block">
            <pre><code v-html="highlightCode(msg.content)"></code></pre>
          </div>
          <div v-else-if="msg.type === 'error'" class="error-message">
            <el-icon><Warning /></el-icon>
            <span>{{ msg.metadata?.error || msg.content }}</span>
          </div>
          <div v-else-if="msg.type === 'thinking'" class="thinking-note">{{ msg.content }}</div>
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

    <!-- 自动滚动到底部 -->
    <div ref="scrollAnchor"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { Loading, Warning } from '@element-plus/icons-vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import type { AiMessage } from '../utils/aiTypes'

interface Props {
  messages: AiMessage[]
  isStreaming?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  isStreaming: false,
})

const scrollAnchor = ref<HTMLDivElement>()
const chartFullscreenVisible = ref(false)
const fullscreenChartHtml = ref('')

// 自动滚动到底部
watch(
  () => [props.messages.length, props.isStreaming],
  async () => {
    await nextTick()
    scrollAnchor.value?.scrollIntoView({ behavior: 'smooth' })
  },
  { deep: true }
)

// 代码高亮（简单实现，后续可集成 highlight.js）
function highlightCode(code: string): string {
  // 这里简单返回，实际可以使用 highlight.js 或其他库
  return code.replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

function renderMarkdown(content: string): string {
  if (!content) return ''
  const html = marked.parse(content, { breaks: true }) as string
  return DOMPurify.sanitize(html)
}

function openChartFullscreen(html: string) {
  fullscreenChartHtml.value = html
  chartFullscreenVisible.value = true
}
</script>

<style scoped>
.message-stream {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
  padding: 16px;
  gap: 8px;
  background-color: var(--el-bg-color);
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
