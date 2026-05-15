/**
 * AI 消息输入框组件
 * 用户输入、发送消息、快捷命令
 */

<template>
  <div class="message-input-area">
    <div class="input-toolbar">
      <!-- 文件上传 -->
      <el-tooltip content="上传文件">
        <el-button link :icon="DocumentCopy" @click="handleUploadClick" />
      </el-tooltip>
      <input
        ref="fileInputRef"
        type="file"
        accept=".csv,.xlsx,.xls"
        style="display: none"
        @change="handleFileChange"
      />

      <!-- 快捷命令菜单 -->
      <el-dropdown>
        <el-button link :icon="More" />
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="cmd in slashCommands"
              :key="cmd.name"
              @click="applySlashCommand(cmd.name)"
            >
              {{ cmd.icon }} /{{ cmd.name }} <span style="color: var(--el-text-color-secondary); font-size: 12px; margin-left: 6px">{{ cmd.description }}</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <!-- 清空对话 -->
      <el-tooltip content="清空对话">
        <el-button link :icon="Delete" @click="clearChat" />
      </el-tooltip>
    </div>

    <!-- 输入框 -->
    <div class="input-wrapper">
      <div class="token-corner" v-if="tokenLabel">
        <div class="token-bar-track">
          <div :class="['token-bar-fill', tokenFillClass]" :style="{ width: `${tokenPercent}%` }"></div>
        </div>
        <span class="token-bar-label">{{ tokenLabel }}</span>
      </div>

      <div v-if="activeSlashCommand" class="cmd-badge">
        <span>{{ commandDisplay(activeSlashCommand) }}</span>
        <button type="button" class="cmd-badge-close" @click="activeSlashCommand = ''">×</button>
      </div>

      <el-input
        v-model="userInput"
        type="textarea"
        :rows="3"
        :maxlength="2000"
        placeholder="输入你的问题或分析需求... 输入 / 调出命令 (Ctrl/Cmd+Enter 发送)"
        @keydown="handleKeydown"
        @input="handleInput"
        class="message-textarea"
      />

      <div v-if="showSlashPanel" class="slash-panel">
        <div class="slash-title">斜杠命令</div>
        <div class="slash-list">
          <button
            v-for="cmd in filteredSlashCommands"
            :key="cmd.name"
            type="button"
            class="slash-item"
            @click="applySlashCommand(cmd.name)"
          >
            <span class="slash-name">{{ cmd.icon }} /{{ cmd.name }}</span>
            <span class="slash-desc">{{ cmd.description }}</span>
          </button>
        </div>
      </div>

      <div class="input-footer">
        <span class="char-count">{{ userInput.length }} / 2000</span>
        <el-button type="primary" :loading="isSending" :disabled="!userInput.trim() || isSending" @click="sendMessage">
            <el-icon><Right /></el-icon>
            发送
          </el-button>
      </div>
    </div>

    <!-- 停止按钮 -->
    <div v-if="isSending" class="stop-button">
      <el-button @click="stopStream">
        <el-icon><VideoPause /></el-icon>
        停止生成
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  DocumentCopy,
  More,
  Delete,
  Right,
  VideoPause,
} from '@element-plus/icons-vue'

interface Props {
  isSending?: boolean
  tokenStats?: {
    inputTokens: number
    outputTokens: number
    sessionTotalInput: number
    sessionTotalOutput: number
    contextWindow: number
    maxOutputTokens: number
  }
}

interface Emits {
  (e: 'send', message: string): void
  (e: 'command', cmd: string): void
  (e: 'sendWithCommand', payload: { command: string; message: string }): void
  (e: 'stop'): void
  (e: 'clear'): void
  (e: 'upload-file', file: File): void
}

const props = withDefaults(defineProps<Props>(), {
  isSending: false,
  tokenStats: () => ({
    inputTokens: 0,
    outputTokens: 0,
    sessionTotalInput: 0,
    sessionTotalOutput: 0,
    contextWindow: 0,
    maxOutputTokens: 0,
  }),
})

const emit = defineEmits<Emits>()

const userInput = ref('')
const activeSlashCommand = ref<SlashCommand['name'] | ''>('')
const fileInputRef = ref<HTMLInputElement | null>(null)

function handleUploadClick() {
  fileInputRef.value?.click()
}

function handleFileChange(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) {
    emit('upload-file', file)
  }
  // reset so same file can be re-selected
  if (fileInputRef.value) fileInputRef.value.value = ''
}

interface SlashCommand {
  name:
    | 'chart'
    | 'sql'
    | 'decile'
    | 'tree'
    | 'kmeans'
    | 'data'
    | 'inset'
    | 'winsorize'
    | 'trimming'
    | 'export'
    | 'report'
    | 'ppt'
    | 'dashboard'
    | 'status'
  icon: string
  description: string
}

const slashCommands: SlashCommand[] = [
  { name: 'chart', icon: '📊', description: '图表分析与可视化' },
  { name: 'sql', icon: '🗄️', description: '执行 SQL 查询分析' },
  { name: 'decile', icon: '📉', description: '十分位分析' },
  { name: 'tree', icon: '🌳', description: '决策树分析' },
  { name: 'kmeans', icon: '🔵', description: 'KMeans 聚类分析' },
  { name: 'data', icon: '🔍', description: '数据概览与质量检查' },
  { name: 'inset', icon: '🩹', description: '缺失值处理建议' },
  { name: 'winsorize', icon: '✂️', description: '缩尾处理异常值' },
  { name: 'trimming', icon: '🔪', description: '区间裁剪异常值' },
  { name: 'export', icon: '📥', description: '导出 Excel' },
  { name: 'report', icon: '📄', description: '生成 Word 报告' },
  { name: 'ppt', icon: '🎯', description: '生成 PPT' },
  { name: 'dashboard', icon: '📊', description: '生成 Dashboard' },
  { name: 'status', icon: '📡', description: '查看状态' },
]

const slashKeyword = computed(() => {
  const text = userInput.value.trimStart()
  if (!text.startsWith('/')) return ''
  return text.slice(1).split(/\s|\n/, 1)[0].toLowerCase()
})

const filteredSlashCommands = computed(() => {
  if (!slashKeyword.value) return slashCommands
  return slashCommands.filter((cmd) => cmd.name.includes(slashKeyword.value))
})

const showSlashPanel = computed(() => {
  if (props.isSending) return false
  const text = userInput.value.trimStart()
  return text.startsWith('/') && filteredSlashCommands.value.length > 0
})

const tokenPercent = computed(() => {
  const total = props.tokenStats.contextWindow || 0
  if (!total) return 0
  return Math.max(0, Math.min(100, Math.round((props.tokenStats.sessionTotalInput / total) * 100)))
})

const tokenFillClass = computed(() => {
  if (tokenPercent.value >= 90) return 'crit'
  if (tokenPercent.value >= 70) return 'warn'
  return ''
})

const tokenLabel = computed(() => {
  if (!props.tokenStats.contextWindow) {
    return `输入 ${props.tokenStats.inputTokens} / 输出 ${props.tokenStats.outputTokens}`
  }
  return `输入 ${props.tokenStats.sessionTotalInput}/${props.tokenStats.contextWindow} · 输出 ${props.tokenStats.sessionTotalOutput}`
})

function applySlashCommand(cmd: SlashCommand['name']) {
  activeSlashCommand.value = cmd
  userInput.value = userInput.value.replace(/^\/\S*\s*/, '')
}

function commandDisplay(cmd: SlashCommand['name']): string {
  const found = slashCommands.find((x) => x.name === cmd)
  return found ? `${found.icon} /${found.name}` : `/${cmd}`
}

function handleInput(v: string) {
  const value = String(v)

  if (value === '/stop' && props.isSending) {
    userInput.value = ''
    emit('stop')
    return
  }

  const mCmdOnly = value.match(/^\/(\w+)\s$/)
  if (mCmdOnly) {
    const found = slashCommands.find((c) => c.name === mCmdOnly[1])
    if (found) {
      applySlashCommand(found.name)
      userInput.value = ''
      return
    }
  }

  const mCmdArgs = value.match(/^\/(\w+)\s+(.+)/)
  if (mCmdArgs) {
    const found = slashCommands.find((c) => c.name === mCmdArgs[1])
    if (found) {
      applySlashCommand(found.name)
      userInput.value = mCmdArgs[2]
      return
    }
  }
}

// 发送消息
function sendMessage() {
  const msg = userInput.value.trim()
  if (!msg && !activeSlashCommand.value) return

  if (msg.startsWith('/')) {
    const found = slashCommands.find((item) => msg.startsWith(`/${item.name}`))
    if (found) {
      const rest = msg.replace(new RegExp(`^/${found.name}\\s*`), '')
      emit('sendWithCommand', { command: found.name, message: rest })
      userInput.value = ''
      activeSlashCommand.value = ''
      return
    }
  }

  if (activeSlashCommand.value) {
    emit('sendWithCommand', { command: activeSlashCommand.value, message: msg })
    userInput.value = ''
    return
  }

  emit('send', msg)
  userInput.value = ''
}

// 清空对话
async function clearChat() {
  await ElMessageBox.confirm('确定要清空对话历史吗？', '清空对话', {
    confirmButtonText: '清空',
    cancelButtonText: '取消',
    type: 'warning',
  })
    .then(() => {
      emit('clear')
      ElMessage.success('已清空对话')
    })
    .catch(() => {})
}

// 键盘事件
function handleKeydown(e: KeyboardEvent) {
  if (showSlashPanel.value && e.key === 'Enter') {
    e.preventDefault()
    const cmd = filteredSlashCommands.value[0]
    if (cmd) applySlashCommand(cmd.name)
    return
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    sendMessage()
  }

  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

// 停止生成
function stopStream() {
  emit('stop')
}
</script>

<style scoped>
.message-input-area {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background-color: var(--el-bg-color);
  border-top: 1px solid var(--el-border-color);
}

.input-toolbar {
  display: flex;
  gap: 4px;
}

.input-toolbar :deep(.el-button) {
  width: 32px;
  height: 32px;
  padding: 0;
}

.input-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
  position: relative;
  padding-top: 18px;
}

.token-corner {
  position: absolute;
  right: 0;
  top: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 58%;
}

.token-bar-track {
  flex: 1;
  min-width: 90px;
  height: 3px;
  background: #e2e8f0;
  border-radius: 2px;
  overflow: hidden;
}

.token-bar-fill {
  height: 100%;
  border-radius: 2px;
  background: #22c55e;
  transition: width .4s ease, background-color .4s ease;
}

.token-bar-fill.warn { background: #f59e0b; }
.token-bar-fill.crit { background: #ef4444; }

.token-bar-label {
  font-size: 11px;
  color: #94a3b8;
  white-space: nowrap;
  user-select: none;
}

.cmd-badge {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--el-color-primary-light-5);
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
  font-size: 12px;
  font-weight: 600;
}

.cmd-badge-close {
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}

.message-textarea {
  resize: none;
}

.message-textarea :deep(.el-textarea__inner) {
  font-size: 13px;
  line-height: 1.4;
  border-radius: 6px;
}

.input-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.slash-panel {
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  background: var(--el-fill-color-blank);
  padding: 8px;
}

.slash-title {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-bottom: 6px;
}

.slash-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.slash-item {
  border: 1px solid transparent;
  background: var(--el-fill-color-light);
  color: var(--el-text-color-primary);
  border-radius: 6px;
  padding: 6px 8px;
  text-align: left;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.slash-item:hover {
  border-color: var(--el-color-primary-light-5);
}

.slash-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--el-color-primary);
}

.slash-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.char-count {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.stop-button {
  text-align: center;
}
</style>
