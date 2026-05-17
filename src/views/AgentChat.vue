<script setup lang="ts">
import { computed, reactive, ref, watch, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Setting, ArrowDown, ArrowUp, ArrowRight, DataLine, Document, DocumentCopy } from '@element-plus/icons-vue'
import { useAgent } from '../composables/useAgent'
import { useSessionStore } from '../stores/sessionStore'
import { useDataStore } from '../stores/dataStore'
import AiSessionSidebar from '../components/AiSessionSidebar.vue'
import AiMessageStream from '../components/AiMessageStream.vue'
import AiMessageInput from '../components/AiMessageInput.vue'
import type { AiModelConfig } from '../utils/aiTypes'

// ────────────────────────────────────────────────────────────
// 会话管理
// ────────────────────────────────────────────────────────────

const sessionStore = useSessionStore()
const dataStore = useDataStore()
const agent = useAgent()
const isStreaming = ref(false)
const rustSessionMap = ref<Record<string, string>>({})

onMounted(() => {
  sessionStore.loadFromStorage()
  // 如果没有会话，创建一个
  if (!sessionStore.currentSession) {
    sessionStore.createSession()
  }
})

const currentSession = computed(() => sessionStore.currentSession)

// ────────────────────────────────────────────────────────────
// 模型配置（从 localStorage）
// ────────────────────────────────────────────────────────────

const STORAGE_CONFIGS = 'bi.ai.model.configs.v1'
const STORAGE_SELECTED = 'bi.ai.model.selected.v1'

const defaultConfigs: AiModelConfig[] = [
  {
    id: 'deepseek',
    provider: 'deepseek',
    displayName: 'DeepSeek',
    apiKey: '',
    baseUrl: 'https://api.deepseek.com',
    model: 'deepseek-chat',
    enabled: true,
    isCustom: false,
    contextWindow: 64000,
    maxOutputTokens: 8192,
    enableThinking: false,
  },
  {
    id: 'openai',
    provider: 'openai',
    displayName: 'OpenAI / ChatGPT',
    apiKey: '',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o-mini',
    enabled: true,
    isCustom: false,
    contextWindow: 128000,
    maxOutputTokens: 16384,
    enableThinking: false,
  },
  {
    id: 'claude',
    provider: 'claude',
    displayName: 'Claude',
    apiKey: '',
    baseUrl: 'https://api.anthropic.com',
    model: 'claude-3-5-haiku-20241022',
    enabled: true,
    isCustom: false,
    contextWindow: 200000,
    maxOutputTokens: 8192,
    enableThinking: false,
  },
]

function loadConfigs(): AiModelConfig[] {
  try {
    const raw = window.localStorage.getItem(STORAGE_CONFIGS)
    if (!raw) return defaultConfigs
    const parsed = JSON.parse(raw) as AiModelConfig[]
    if (!Array.isArray(parsed) || parsed.length === 0) return defaultConfigs
    return parsed
  } catch {
    return defaultConfigs
  }
}

const modelConfigs = ref<AiModelConfig[]>(loadConfigs())
const selectedModelId = ref(window.localStorage.getItem(STORAGE_SELECTED) || modelConfigs.value[0]?.id || '')
const settingsVisible = ref(false)
const sidebarCollapsed = ref(false)
const modelConfigCollapsed = ref(false)
const sessionListCollapsed = ref(false)
const datasourceCollapsed = ref(false)
type DatasourceChoice = 'dataset' | 'upload'
const uploadedFileInfo = ref<{ name: string; size: number; file: File } | null>(null)
// 'dataset' = BI已加载数据集, 'upload' = 手动上传文件
const selectedDatasources = ref<DatasourceChoice[]>(['dataset'])
const tokenStats = ref({
  inputTokens: 0,
  outputTokens: 0,
  sessionTotalInput: 0,
  sessionTotalOutput: 0,
  contextWindow: 0,
  maxOutputTokens: 0,
})
const streamAbortController = ref<AbortController | null>(null)

async function ensureRustSessionId(): Promise<string> {
  const localSessionId = sessionStore.currentSessionId
  if (!localSessionId) {
    throw new Error('当前没有本地会话')
  }

  if (rustSessionMap.value[localSessionId]) {
    return rustSessionMap.value[localSessionId]
  }

  const rustSessionId = await agent.createSession(selectedModelId.value || 'openai')
  rustSessionMap.value[localSessionId] = rustSessionId
  return rustSessionId
}

const customForm = reactive({
  displayName: '',
  baseUrl: '',
  model: '',
  apiKey: '',
  contextWindow: null as number | null,
  maxOutputTokens: null as number | null,
  enableThinking: false,
})

const enabledModels = computed(() => modelConfigs.value.filter((m) => m.enabled))
const selectedModel = computed(() => modelConfigs.value.find((m) => m.id === selectedModelId.value) || null)

watch(
  modelConfigs,
  (list) => {
    window.localStorage.setItem(STORAGE_CONFIGS, JSON.stringify(list))
    if (list.length === 0) {
      selectedModelId.value = ''
      return
    }
    const existing = list.find((m) => m.id === selectedModelId.value)
    if (!existing) {
      selectedModelId.value = list[0].id
    }
  },
  { deep: true }
)

watch(selectedModelId, (id) => {
  window.localStorage.setItem(STORAGE_SELECTED, id)
  sessionStore.setSelectedModel(sessionStore.currentSessionId, id)
})

// ────────────────────────────────────────────────────────────
// 数据源选择方法
// ────────────────────────────────────────────────────────────

function setDatasourceSelected(source: DatasourceChoice, checked: boolean) {
  const exists = selectedDatasources.value.includes(source)
  if (checked && !exists) {
    selectedDatasources.value.push(source)
  } else if (!checked && exists) {
    selectedDatasources.value = selectedDatasources.value.filter((x) => x !== source)
  }
}

// ────────────────────────────────────────────────────────────
// 模型配置方法
// ────────────────────────────────────────────────────────────

function selectModel(modelId: string) {
  selectedModelId.value = modelId
}

function saveModel(configId: string) {
  const cfg = modelConfigs.value.find((m) => m.id === configId)
  if (!cfg) return
  if (!cfg.baseUrl.trim()) {
    ElMessage.warning('API Base URL 不能为空')
    return
  }
  if (!cfg.model.trim()) {
    ElMessage.warning('Model ID 不能为空')
    return
  }
  if (!cfg.apiKey.trim()) {
    ElMessage.warning('API Key 不能为空')
    return
  }

  ElMessage.success(`已保存 ${cfg.displayName} 配置`)
}

function clearModel(configId: string) {
  const cfg = modelConfigs.value.find((m) => m.id === configId)
  if (!cfg) return
  cfg.apiKey = ''
  ElMessage.success(`已清空 ${cfg.displayName} API Key`)
}

function addCustomModel() {
  if (!customForm.displayName.trim()) {
    ElMessage.warning('模型名称（显示用）不能为空')
    return
  }
  if (!customForm.baseUrl.trim()) {
    ElMessage.warning('API Base URL 不能为空')
    return
  }
  if (!customForm.model.trim()) {
    ElMessage.warning('Model ID 不能为空')
    return
  }
  if (!customForm.apiKey.trim()) {
    ElMessage.warning('API Key 不能为空')
    return
  }

  const id = `custom_${customForm.displayName.trim().toLowerCase().replace(/\s+/g, '_')}_${Date.now()}`
  modelConfigs.value.unshift({
    id,
    provider: id,
    displayName: customForm.displayName.trim(),
    apiKey: customForm.apiKey.trim(),
    baseUrl: customForm.baseUrl.trim(),
    model: customForm.model.trim(),
    enabled: true,
    isCustom: true,
    contextWindow: customForm.contextWindow,
    maxOutputTokens: customForm.maxOutputTokens,
    enableThinking: customForm.enableThinking,
  })

  customForm.displayName = ''
  customForm.baseUrl = ''
  customForm.model = ''
  customForm.apiKey = ''
  customForm.contextWindow = null
  customForm.maxOutputTokens = null
  customForm.enableThinking = false
  ElMessage.success('自定义模型已添加')
}

function removeCustomModel(configId: string) {
  const idx = modelConfigs.value.findIndex((m) => m.id === configId)
  if (idx < 0) return
  modelConfigs.value.splice(idx, 1)
  ElMessage.success('已删除自定义模型')
}

function resetToDefaults() {
  modelConfigs.value = JSON.parse(JSON.stringify(defaultConfigs)) as AiModelConfig[]
  selectedModelId.value = modelConfigs.value[0]?.id || ''
  ElMessage.success('已恢复默认模型配置')
}

function pushChartError(errorMsg: string) {
  sessionStore.addMessage('assistant', errorMsg, 'error', {
    error: errorMsg,
  })
  ElMessage.error(errorMsg)
}

function buildChartRequestFromCurrentData() {
  const payload = dataStore.payload
  if (!payload || !Array.isArray(payload.rows) || payload.rows.length === 0) {
    throw new Error('当前没有可用于制图的数据，请先加载或切换数据集。')
  }

  const columnNames = payload.columns.map((col) => col.name).filter(Boolean)
  const categoricalColumns = dataStore.categoricalColumns
  const dateColumns = dataStore.dateColumns
  const numericColumns = dataStore.numericColumns

  const xCol = categoricalColumns[0] || dateColumns[0] || columnNames[0]
  const yCol = numericColumns[0] || columnNames.find((name) => name !== xCol) || columnNames[1]

  if (!xCol || !yCol) {
    throw new Error('当前数据列不足，至少需要一列分类/日期列和一列数值列。')
  }

  return {
    rows: payload.rows,
    xCol,
    yCol,
    totalRows: payload.total_rows,
  }
}

// ────────────────────────────────────────────────────────────
// 消息处理
// ────────────────────────────────────────────────────────────

async function handleSendMessage(
  message: string,
  command = '',
  options?: {
    suppressUserMessage?: boolean
    excelTables?: string[]
    excelFilename?: string
    reportTitle?: string
    reportSections?: Record<string, any>[]
    pptTitle?: string
    pptSlides?: Record<string, any>[]
    dashboardName?: string
    dashboardWidgets?: Record<string, any>[]
  }
) {
  if (!selectedModel.value) {
    ElMessage.warning('请先选择一个模型')
    return
  }

  // 添加用户消息
  if (!options?.suppressUserMessage) {
    const userDisplayMessage = command ? `/${command} ${message}`.trim() : message
    sessionStore.addMessage('user', userDisplayMessage)
  }

  isStreaming.value = true
  
  try {
    // 检查是否是图表生成命令
    if (command === 'chart' || message.startsWith('/chart')) {
      await handleChartGeneration(message)
      return
    }
    
    const rustSessionId = await ensureRustSessionId()
    const assistantMsg = sessionStore.addMessage('assistant', '', 'text_delta')

    await agent.chatStream(rustSessionId, message, (event) => {
      if (event.type === 'text_delta' && event.content) {
        sessionStore.appendToMessage(assistantMsg.id, event.content)
      } else if (event.type === 'text' && event.content) {
        sessionStore.appendToMessage(assistantMsg.id, event.content)
        sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
      } else if (event.type === 'thinking') {
        sessionStore.addMessage('assistant', event.content || '思考中...', 'thinking')
      } else if (event.type === 'tool_start') {
        sessionStore.addMessage('assistant', '', 'tool_start', {
          toolName: event.tool || event.tool_name || 'tool',
          display: event.display || '执行中',
        })
      } else if (event.type === 'tool_result') {
        const resultText = event.content || event.message || '工具执行完成'
        sessionStore.addMessage('assistant', resultText, 'text')
      } else if (event.type === 'reasoning' && event.content) {
        sessionStore.addMessage('assistant', event.content, 'thinking')
      } else if (event.type === 'error') {
        const err = event.error || event.message || '未知错误'
        sessionStore.updateMessage(assistantMsg.id, {
          type: 'error',
          metadata: { error: err },
        })
      } else if (event.type === 'done') {
        sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
        isStreaming.value = false
      }
    }, {
      command,
      provider: selectedModel.value.provider,
      model: selectedModel.value.model,
      apiKey: selectedModel.value.apiKey,
      baseUrl: selectedModel.value.baseUrl,
    })

    isStreaming.value = false
    
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AgentChat] handleSendMessage failed:', errorMsg)
    ElMessage.error(`发送失败: ${errorMsg}`)
    isStreaming.value = false
  }
}

// 处理图表生成
async function handleChartGeneration(message: string) {
  try {
    // 解析图表类型和数据
    // 示例: /chart Bar_Chart {"x": "country", "y": "gdp"}
    const normalized = message.startsWith('/chart')
      ? message.replace('/chart', '').trim()
      : message.trim()
    const [rawChartType, ...titleParts] = normalized.split(/\s+/)
    const chartType = rawChartType || 'Bar_Chart'
    const chartTitle = titleParts.join(' ').trim() || `${chartType} 图表`

    const chartRequest = buildChartRequestFromCurrentData()

    const assistantMsg = sessionStore.addMessage('assistant', '正在执行图表工作流...', 'thinking')

    await agent.chartWorkflow(
      chartType,
      chartRequest.xCol,
      chartRequest.yCol,
      (event) => {
        if (event.type === 'thinking') {
          sessionStore.addMessage('assistant', event.content || '正在准备图表...', 'thinking')
        } else if (event.type === 'tool_start') {
          sessionStore.addMessage('assistant', '', 'tool_start', {
            toolName: event.tool || event.tool_name || 'tool',
            display: event.display || '执行中',
          })
        } else if (event.type === 'tool_result') {
          sessionStore.addMessage('assistant', event.content || event.message || '工具执行完成', 'text')
        } else if (event.type === 'chart_html' && event.html) {
          sessionStore.addMessage('assistant', '已生成图表', {
            type: 'chart_generated',
            html: event.html,
            chartType: event.chart_type || chartType,
            meta: event.meta || {
              xCol: chartRequest.xCol,
              yCol: chartRequest.yCol,
              totalRows: chartRequest.totalRows,
            },
          })
        } else if (event.type === 'error') {
          const err = event.error || event.message || '未知错误'
          sessionStore.updateMessage(assistantMsg.id, {
            type: 'error',
            metadata: { error: err },
          })
        } else if (event.type === 'done') {
          sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
        }
      },
      chartTitle
    )
    
    ElMessage.success('图表生成成功！')
    isStreaming.value = false
    
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AgentChat] Chart generation failed:', errorMsg)
    pushChartError(`图表生成失败: ${errorMsg}`)
    isStreaming.value = false
  }
}

async function handleSendWithCommand(payload: { command: string; message: string }) {
  await handleSendMessage(payload.message, payload.command)
}

async function handleCommand(command: string) {
  switch (command) {
    case 'clear':
      handleClearChat()
      break
    case 'help':
      showHelp()
      break
    default:
      ElMessage.info(`未知命令: /${command}`)
  }
}

function handleStopStream() {
  if (streamAbortController.value) {
    streamAbortController.value.abort()
    streamAbortController.value = null
  }
  isStreaming.value = false
  ElMessage.info('已停止生成')
}

function handleClearChat() {
  if (currentSession.value) {
    currentSession.value.messages = []
    ElMessage.success('已清空对话')
  }
}

async function handleUploadFile(file: File) {
  try {
    // ✅ 保存文件信息到前端状态
    uploadedFileInfo.value = {
      name: file.name,
      size: file.size,
      file,
    }
    
    // ✅ 自动选中上传的数据源
    if (!selectedDatasources.value.includes('upload')) {
      selectedDatasources.value.push('upload')
    }
    
    // ✅ 将文件保存到临时目录
    const { appDataDir } = await import('@tauri-apps/api/path')
    const { writeFile, mkdir, exists } = await import('@tauri-apps/plugin-fs')
    
    // 获取应用数据目录
    const appDataPath = await appDataDir()
    const tempDirPath = `${appDataPath}/temp_uploads`
    const tempFilePath = `${tempDirPath}/temp_upload_${Date.now()}_${file.name}`
    
    // ✅ 确保目录存在
    if (!(await exists(tempDirPath))) {
      await mkdir(tempDirPath, { recursive: true })
    }
    
    // 读取文件并保存到临时位置
    const fileBytes = await file.arrayBuffer()
    await writeFile(tempFilePath, new Uint8Array(fileBytes))
    
    // ✅ 调用后端 load_file 命令加载数据到 GLOBAL_DF
    const { invoke } = await import('@tauri-apps/api/core')
    const result: any = await invoke('load_file', {
      path: tempFilePath,
      skipHead: 0,
      skipTail: 0,
      headerRow: -1,
      headerLocked: false,
    })
    
    if (result.ok) {
      ElMessage.success(`文件 "${file.name}" 已加载，共 ${result.data?.total_rows || 0} 行`)
    } else {
      ElMessage.error(result.error || '文件加载失败')
      uploadedFileInfo.value = null
    }
  } catch (error) {
    console.error('[AgentChat] handleUploadFile failed:', error)
    ElMessage.error(`上传失败: ${error instanceof Error ? error.message : String(error)}`)
    uploadedFileInfo.value = null
  }
}

function handleOutlineAction(payload: {
  action: 'confirm' | 'revise' | 'cancel'
  outlineType: 'excel' | 'report' | 'ppt' | 'dashboard'
  tables?: string[]
  filename?: string
  title?: string
  sections?: Record<string, any>[]
  slides?: Record<string, any>[]
  name?: string
  widgets?: Record<string, any>[]
}) {
  console.log('[AgentChat] Outline action:', payload)
  // TODO: 实现大纲操作
}

function showHelp() {
  ElMessage.info({
    message: '可用命令：\n/clear - 清空对话\n/help - 显示帮助',
    duration: 5000,
  })
}

</script>

<template>
  <div class="ai-analysis-container">
    <!-- 左侧边栏 -->
    <aside :class="['ai-sidebar', { collapsed: sidebarCollapsed }]">
      <!-- 侧边栏折叠按钮 -->
      <div v-if="sidebarCollapsed" class="sidebar-collapsed-only" @click="sidebarCollapsed = false">
        <el-icon><ArrowRight /></el-icon>
      </div>

      <div v-show="!sidebarCollapsed" class="sidebar-body">
        <!-- 数据源选择 -->
        <el-card :class="['panel-card', { collapsed: datasourceCollapsed }]" shadow="never">
          <template #header>
            <div class="panel-header">
              <div class="panel-title">数据源</div>
              <el-button link class="panel-collapse-btn" @click="datasourceCollapsed = !datasourceCollapsed">
                <el-icon>
                  <component :is="datasourceCollapsed ? ArrowDown : ArrowUp" />
                </el-icon>
              </el-button>
            </div>
          </template>

          <div v-show="!datasourceCollapsed" class="datasource-panel-body">
            <!-- 当前 BI 数据集 -->
            <div class="ds-section-label">选择发送给 AI 的数据源</div>

            <!-- 只有一个数据源时直接显示，都存在时显示 checkbox -->
            <div v-if="dataStore.hasData" :class="['ds-item', { 'ds-item--active': !uploadedFileInfo || selectedDatasources.includes('dataset') }]">
              <el-checkbox
                v-if="uploadedFileInfo"
                :model-value="selectedDatasources.includes('dataset')"
                @change="(v: boolean) => setDatasourceSelected('dataset', v)"
              />
              <el-icon class="ds-icon"><DataLine /></el-icon>
              <div class="ds-info">
                <div class="ds-name">{{ dataStore.datasets.find(d => d.id === dataStore.activeDatasetId)?.name || dataStore.activeDatasetId }}</div>
                <div class="ds-meta">{{ dataStore.payload?.total_rows?.toLocaleString() }} 行 · {{ dataStore.columnNames.length }} 列</div>
              </div>
              <el-tag size="small" type="success">BI数据</el-tag>
            </div>
            <div v-else class="ds-empty">未加载数据集，请前往「数据载入」页面</div>

            <!-- 手动上传的文件 -->
            <div v-if="uploadedFileInfo" :class="['ds-item', { 'ds-item--active': selectedDatasources.includes('upload') }]" style="margin-top: 6px">
              <el-checkbox
                v-if="dataStore.hasData"
                :model-value="selectedDatasources.includes('upload')"
                @change="(v: boolean) => setDatasourceSelected('upload', v)"
              />
              <el-icon class="ds-icon"><Document /></el-icon>
              <div class="ds-info">
                <div class="ds-name">{{ uploadedFileInfo.name }}</div>
                <div class="ds-meta">{{ (uploadedFileInfo.size / 1024).toFixed(1) }} KB · 已上传</div>
              </div>
              <el-button
                link
                size="small"
                style="margin-left: auto"
                @click="uploadedFileInfo = null; selectedDatasources = selectedDatasources.filter((x) => x !== 'upload')"
              >×</el-button>
            </div>
            <div v-else class="ds-empty" style="margin-top: 6px">点击输入框左上角 <el-icon style="vertical-align: middle"><DocumentCopy /></el-icon> 可额外上传文件</div>
          </div>
        </el-card>

        <!-- 模型选择 -->
        <el-card :class="['panel-card', 'model-panel-card', { collapsed: modelConfigCollapsed }]" shadow="never">
          <template #header>
            <div class="panel-header">
              <div class="panel-title">模型</div>
              <el-button link class="panel-collapse-btn" @click="modelConfigCollapsed = !modelConfigCollapsed">
                <el-icon>
                  <component :is="modelConfigCollapsed ? ArrowDown : ArrowUp" />
                </el-icon>
              </el-button>
            </div>
          </template>

          <div v-show="!modelConfigCollapsed" class="model-panel-body">
            <div class="model-label">选择模型</div>
            <div class="model-select-row">
              <div
                :class="[
                  'model-cards-container',
                  { single: enabledModels.length === 1 }
                ]"
              >
                <button
                  v-for="model in enabledModels"
                  :key="model.id"
                  :class="['model-card-button', { active: selectedModelId === model.id }]"
                  @click="selectModel(model.id)"
                >
                  <span class="model-card-name">{{ model.displayName }}</span>
                  <span v-if="selectedModelId === model.id" class="model-card-badge">✓</span>
                </button>
              </div>
              <el-button class="model-settings-btn" @click="settingsVisible = true">
                <el-icon><Setting /></el-icon>
              </el-button>
            </div>

            <!-- Token 统计 -->
            <el-alert
              v-if="tokenStats.sessionTotalInput > 0 || tokenStats.sessionTotalOutput > 0"
              type="info"
              :closable="false"
              class="model-alert"
            >
              <template #default>
                <div style="font-size: 12px;">
                  <div>输入: {{ tokenStats.sessionTotalInput }} tokens</div>
                  <div>输出: {{ tokenStats.sessionTotalOutput }} tokens</div>
                </div>
              </template>
            </el-alert>
          </div>
        </el-card>

        <!-- 会话列表 -->
        <el-card :class="['panel-card', 'session-panel-card', { collapsed: sessionListCollapsed }]" shadow="never">
          <template #header>
            <div class="panel-header">
              <div class="panel-title">对话列表</div>
              <el-button link class="panel-collapse-btn" @click="sessionListCollapsed = !sessionListCollapsed">
                <el-icon>
                  <component :is="sessionListCollapsed ? ArrowDown : ArrowUp" />
                </el-icon>
              </el-button>
            </div>
          </template>
          <div v-show="!sessionListCollapsed" class="session-panel-body">
            <AiSessionSidebar />
          </div>
        </el-card>
      </div>
    </aside>

    <!-- 右侧：消息流 + 输入框 -->
    <main class="ai-main">
      <div class="chat-area">
        <!-- 消息流 -->
        <AiMessageStream
          :messages="currentSession?.messages || []"
          :is-streaming="isStreaming"
          @outline-action="handleOutlineAction"
        />

        <!-- 消息输入框 -->
        <AiMessageInput
          :is-sending="isStreaming"
          :token-stats="tokenStats"
          @send="handleSendMessage"
          @send-with-command="handleSendWithCommand"
          @command="handleCommand"
          @stop="handleStopStream"
          @clear="handleClearChat"
          @upload-file="handleUploadFile"
        />
      </div>
    </main>

    <el-dialog v-model="settingsVisible" title="模型参数配置" width="600px" top="6vh">
      <div class="settings-scroll">
        <el-card class="model-card" shadow="never" v-for="cfg in modelConfigs" :key="cfg.id">
          <template #header>
            <div class="model-card-header">
              <strong>{{ cfg.displayName }}</strong>
              <div class="model-card-actions">
                <el-switch v-model="cfg.enabled" active-text="启用" inactive-text="停用" />
                <el-button v-if="cfg.isCustom" type="danger" link @click="removeCustomModel(cfg.id)">删除</el-button>
              </div>
            </div>
          </template>

          <el-form label-width="120px" size="small">
            <el-form-item label="API Key">
              <el-input v-model="cfg.apiKey" type="password" show-password placeholder="sk-..." />
            </el-form-item>
            <el-form-item label="Base URL">
              <el-input v-model="cfg.baseUrl" placeholder="https://api.example.com" />
            </el-form-item>
            <el-form-item label="Model ID">
              <el-input v-model="cfg.model" placeholder="gpt-4o-mini / deepseek-chat / claude-..." />
            </el-form-item>
            <el-form-item label="上下文窗口">
              <el-input-number v-model="cfg.contextWindow" :min="1" :max="5000000" />
            </el-form-item>
            <el-form-item label="最大输出">
              <el-input-number v-model="cfg.maxOutputTokens" :min="1" :max="5000000" />
            </el-form-item>
            <el-form-item label="思考模式">
              <el-checkbox v-model="cfg.enableThinking">启用思考模式</el-checkbox>
            </el-form-item>
            <el-form-item>
              <div class="form-actions">
                <el-button type="danger" plain @click="clearModel(cfg.id)">清空 Key</el-button>
                <el-button type="primary" @click="saveModel(cfg.id)">保存</el-button>
              </div>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card class="model-card" shadow="never">
          <template #header>
            <strong>添加自定义模型</strong>
          </template>
          <el-form label-width="120px" size="small">
            <el-form-item label="模型名称（显示用）">
              <el-input v-model="customForm.displayName" placeholder="如 glm-5-external" />
            </el-form-item>
            <el-form-item label="API Base URL">
              <el-input v-model="customForm.baseUrl" placeholder="https://api.deepseek.com" />
            </el-form-item>
            <el-form-item label="Model ID">
              <el-input v-model="customForm.model" placeholder="deepseek-chat" />
            </el-form-item>
            <el-form-item label="API Key">
              <el-input v-model="customForm.apiKey" type="password" show-password placeholder="sk-..." />
            </el-form-item>
            <el-form-item label="上下文窗口">
              <el-input-number v-model="customForm.contextWindow" :min="1" :max="5000000" />
            </el-form-item>
            <el-form-item label="最大输出">
              <el-input-number v-model="customForm.maxOutputTokens" :min="1" :max="5000000" />
            </el-form-item>
            <el-form-item label="思考模式">
              <el-checkbox v-model="customForm.enableThinking">启用思考模式</el-checkbox>
            </el-form-item>
            <el-form-item>
              <div class="form-actions">
                <el-button @click="resetToDefaults">恢复默认</el-button>
                <el-button type="primary" @click="addCustomModel">添加模型</el-button>
              </div>
            </el-form-item>
          </el-form>
        </el-card>
      </div>
    </el-dialog>
  </div>
</template>

<style scoped>
.ai-analysis-container {
  height: 100%;
  display: flex;
  gap: 12px;
  background-color: var(--el-bg-color);
}

.ai-sidebar {
  width: 300px;
  min-width: 300px;
  max-width: 300px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-x: hidden;
  overflow-y: auto;
  transition: width 0.24s ease, min-width 0.24s ease, max-width 0.24s ease;
}

.ai-sidebar.collapsed {
  width: 28px;
  min-width: 28px;
  max-width: 28px;
  overflow: hidden;
}

.sidebar-header {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 6px 0 8px;
  border-radius: 8px;
  background: var(--el-fill-color-light);
  border: 1px solid var(--el-border-color-light);
  flex: 0 0 auto;
}

.datasource-panel-body {
  font-size: 13px;
}

.ds-section-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-bottom: 6px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.ds-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  background: var(--el-fill-color);
  border: 1px solid var(--el-border-color-lighter);
}

.ds-item--active {
  border-color: var(--el-color-success-light-5);
  background: var(--el-color-success-light-9);
}

.ds-icon {
  font-size: 16px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.ds-info {
  flex: 1;
  min-width: 0;
}

.ds-name {
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--el-text-color-primary);
}

.ds-meta {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-top: 2px;
}

.ds-empty {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
  padding: 4px 2px;
  line-height: 1.5;
}

.sidebar-collapsed-only {
  width: 100%;
  display: flex;
  justify-content: center;
  padding-top: 10px;
  cursor: pointer;
  color: var(--el-text-color-secondary);
  font-size: 24px;
  line-height: 1;
  height: 100%;
  user-select: none;
}

.sidebar-collapsed-only:hover {
  color: var(--el-color-primary);
}

.sidebar-header-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--el-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-collapse-btn {
  padding: 0;
  min-height: 0;
}

.sidebar-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 0 0 auto;
  overflow: visible;
  padding-right: 2px;
}

.ai-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.chat-area {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: white;
  border-radius: 6px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.08);
}

.panel-card {
  height: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.panel-card :deep(.el-card__body) {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.datasource-panel-body {
  min-height: 0;
}

.model-panel-body {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.model-panel-card {
  flex: 0 0 auto;
}

.session-panel-card {
  flex: 0 0 auto;
  min-height: 0;
}

.session-panel-card.collapsed {
  flex: 0 0 auto;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.panel-title {
  font-size: 15px;
  font-weight: 700;
}

.panel-collapse-btn {
  padding: 0;
  min-height: 0;
}

.panel-collapse-btn :deep(.el-icon) {
  font-size: 14px;
}

.session-panel-body {
  overflow: visible;
}

.model-alert {
  margin-top: 8px;
}

.model-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--el-text-color-regular);
  margin-bottom: 6px;
}

.model-select-row {
  display: flex;
  gap: 6px;
  align-items: flex-start;
  margin-bottom: 8px;
}

.model-cards-container {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 4px;
}

.model-cards-container.single {
  grid-template-columns: 1fr;
}

.model-card-button {
  height: 44px;
  padding: 6px 10px;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  background-color: var(--el-fill-color-blank);
  cursor: pointer;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: relative;

  &:hover {
    border-color: var(--el-color-primary-light-3);
    background-color: var(--el-color-primary-light-9);
  }

  &.active {
    border-color: var(--el-color-primary);
    background-color: var(--el-color-primary-light-9);
    box-shadow: 0 0 0 1px rgba(94, 124, 224, 0.15);
  }
}

.model-card-name {
  font-size: 11px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  flex: 1;
  line-height: 1.2;
}

.model-card-button.active .model-card-badge {
  display: flex;
}

.model-card-badge {
  display: none;
  width: 14px;
  height: 14px;
  background-color: var(--el-color-primary);
  color: white;
  border-radius: 50%;
  align-items: center;
  justify-content: center;
  font-size: 9px;
  font-weight: bold;
  margin-left: 4px;
}

.model-settings-btn {
  width: 30px;
  height: 30px;
  min-height: 30px;
  font-size: 13px;
  border: 1px solid var(--el-border-color);
  flex: 0 0 30px;
}

.model-settings-btn :deep(.el-icon) {
  font-size: 14px;
}

.model-info-table {
  margin-top: 4px;
}

.settings-scroll {
  max-height: 70vh;
  overflow: auto;
  display: grid;
  gap: 12px;
}

.model-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.model-card-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.form-actions {
  width: 100%;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 980px) {
  .ai-analysis-container {
    flex-direction: column;
  }

  .ai-sidebar {
    width: 100%;
    min-width: 0;
    max-width: none;
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
