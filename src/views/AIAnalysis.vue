<script setup lang="ts">
import { computed, reactive, ref, watch, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Setting, ArrowLeft, ArrowDown, ArrowUp, DataLine, Document, DocumentCopy } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { useSessionStore } from '../stores/sessionStore'
import { useDataStore } from '../stores/dataStore'
import AiSessionSidebar from '../components/AiSessionSidebar.vue'
import AiMessageStream from '../components/AiMessageStream.vue'
import AiMessageInput from '../components/AiMessageInput.vue'
import type { AiEvent, AiModelConfig } from '../utils/aiTypes'
import type { ChartPayload } from '../utils/chartTypes'

// ────────────────────────────────────────────────────────────
// 会话管理
// ────────────────────────────────────────────────────────────

const sessionStore = useSessionStore()
const dataStore = useDataStore()
const isStreaming = ref(false)

onMounted(() => {
  sessionStore.loadFromStorage()
  // 如果没有会话，创建一个
  if (!sessionStore.currentSession) {
    sessionStore.createSession()
  }

  void bootstrapPythonAgent()
})

const currentSession = computed(() => sessionStore.currentSession)

// ────────────────────────────────────────────────────────────
// 模型配置（从 localStorage）
// ────────────────────────────────────────────────────────────

const STORAGE_CONFIGS = 'bi.ai.model.configs.v1'
const STORAGE_SELECTED = 'bi.ai.model.selected.v1'
const STORAGE_REMOTE_SESSION_MAP = 'bi.ai.remote.session.map.v1'
const STORAGE_REMOTE_DS_SYNC_MAP = 'bi.ai.remote.datasource.sync.v1'

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

function loadRemoteSessionMap(): Record<string, string> {
  try {
    const raw = window.localStorage.getItem(STORAGE_REMOTE_SESSION_MAP)
    if (!raw) return {}
    const parsed = JSON.parse(raw) as Record<string, string>
    if (!parsed || typeof parsed !== 'object') return {}
    return parsed
  } catch {
    return {}
  }
}

function saveRemoteSessionMap() {
  window.localStorage.setItem(STORAGE_REMOTE_SESSION_MAP, JSON.stringify(remoteSessionMap.value))
}

function loadRemoteDataSyncMap(): Record<string, string> {
  try {
    const raw = window.localStorage.getItem(STORAGE_REMOTE_DS_SYNC_MAP)
    if (!raw) return {}
    const parsed = JSON.parse(raw) as Record<string, string>
    if (!parsed || typeof parsed !== 'object') return {}
    return parsed
  } catch {
    return {}
  }
}

function saveRemoteDataSyncMap() {
  window.localStorage.setItem(STORAGE_REMOTE_DS_SYNC_MAP, JSON.stringify(remoteDataSyncMap.value))
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
const pythonAgentBaseUrl = ref('')
const pythonAgentToken = ref('')
const pythonSessionId = ref('')
const pythonAgentReady = ref(false)
const pythonAgentLoading = ref(false)
const pythonAgentStatus = ref<{
  running: boolean
  port: number
  base_url: string
  auth_token: string
  python_bin: string
  app_dir: string
  pid?: number | null
} | null>(null)
const pythonServiceCollapsed = ref(false)
const tokenStats = ref({
  inputTokens: 0,
  outputTokens: 0,
  sessionTotalInput: 0,
  sessionTotalOutput: 0,
  contextWindow: 0,
  maxOutputTokens: 0,
})
const remoteSessionMap = ref<Record<string, string>>({})
const remoteDataSyncMap = ref<Record<string, string>>({})
const streamAbortController = ref<AbortController | null>(null)

remoteSessionMap.value = loadRemoteSessionMap()
remoteDataSyncMap.value = loadRemoteDataSyncMap()

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
  if (pythonAgentReady.value && pythonSessionId.value) {
    void ensureRemoteModelReady(id)
  }
})

watch(
  () => sessionStore.currentSessionId,
  (localSessionId) => {
    if (!localSessionId || !pythonAgentReady.value) return
    void bindRemoteSessionForLocal(localSessionId)
  },
  { immediate: true }
)

watch(
  () => sessionStore.sessions.map((s) => s.sessionId).join('|'),
  () => {
    const existing = new Set(sessionStore.sessions.map((s) => s.sessionId))
    let changed = false
    Object.keys(remoteSessionMap.value).forEach((localId) => {
      if (!existing.has(localId)) {
        delete remoteSessionMap.value[localId]
        delete remoteDataSyncMap.value[localId]
        changed = true
      }
    })
    if (changed) {
      saveRemoteSessionMap()
      saveRemoteDataSyncMap()
    }
  },
  { immediate: true }
)

watch(
  () => {
    const payload = dataStore.payload
    const cols = payload?.columns.map((c) => `${c.name}:${c.dtype}`).join('|') || ''
    const rows = payload?.rows.length || 0
    const total = payload?.total_rows || 0
    return `${dataStore.activeDatasetId}|${cols}|${rows}|${total}`
  },
  () => {
    if (!pythonAgentReady.value || !sessionStore.currentSessionId) return
    void ensureRemoteDatasourceBound(sessionStore.currentSessionId)
  }
)

// ────────────────────────────────────────────────────────────
// 模型配置方法
// ────────────────────────────────────────────────────────────

function selectModel(modelId: string) {
  selectedModelId.value = modelId
}

async function bootstrapPythonAgent() {
  try {
    const status = await invoke<{
      running: boolean
      port: number
      base_url: string
      auth_token: string
      python_bin: string
      app_dir: string
      pid?: number | null
    }>('start_python_agent')

    pythonAgentBaseUrl.value = status.base_url || ''
    pythonAgentToken.value = status.auth_token || ''
    pythonAgentStatus.value = status
    pythonAgentReady.value = true

    const localSessionId = sessionStore.currentSessionId
    if (localSessionId) {
      await bindRemoteSessionForLocal(localSessionId)
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AIAnalysis] bootstrapPythonAgent failed:', errorMsg)
    ElMessage.error(`Python Agent 启动失败:\n${errorMsg}`)
  }
}

async function startPythonAgent() {
  pythonAgentLoading.value = true
  try {
    const status = await invoke<{
      running: boolean
      port: number
      base_url: string
      auth_token: string
      python_bin: string
      app_dir: string
      pid?: number | null
    }>('start_python_agent')

    pythonAgentBaseUrl.value = status.base_url || ''
    pythonAgentToken.value = status.auth_token || ''
    pythonAgentStatus.value = status
    pythonAgentReady.value = true

    ElMessage.success(`Python Agent 已启动 (端口: ${status.port})`)

    const localSessionId = sessionStore.currentSessionId
    if (localSessionId) {
      await bindRemoteSessionForLocal(localSessionId)
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AIAnalysis] startPythonAgent failed:', errorMsg)
    ElMessage.error(`启动失败: ${errorMsg}`)
  } finally {
    pythonAgentLoading.value = false
  }
}

async function stopPythonAgent() {
  pythonAgentLoading.value = true
  try {
    const status = await invoke<{
      running: boolean
      port: number
      base_url: string
      auth_token: string
      python_bin: string
      app_dir: string
      pid?: number | null
    }>('stop_python_agent')

    pythonAgentStatus.value = status
    pythonAgentReady.value = status.running
    if (!status.running) {
      pythonAgentBaseUrl.value = ''
      pythonAgentToken.value = ''
    }

    ElMessage.success('Python Agent 已停止')
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AIAnalysis] stopPythonAgent failed:', errorMsg)
    ElMessage.error(`停止失败: ${errorMsg}`)
  } finally {
    pythonAgentLoading.value = false
  }
}

async function checkPythonAgentHealth() {
  pythonAgentLoading.value = true
  try {
    const status = await invoke<{
      running: boolean
      port: number
      base_url: string
      auth_token: string
      python_bin: string
      app_dir: string
      pid?: number | null
    }>('python_agent_health')

    pythonAgentStatus.value = status
    if (status.running) {
      ElMessage.success(`Python Agent 运行正常 (端口: ${status.port}, PID: ${status.pid || 'N/A'})`)
    } else {
      ElMessage.warning('Python Agent 未运行')
    }
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    console.error('[AIAnalysis] checkPythonAgentHealth failed:', errorMsg)
    ElMessage.warning(`检测失败: ${errorMsg}`)
  } finally {
    pythonAgentLoading.value = false
  }
}

function withSidecarHeaders(headers?: HeadersInit): Headers {
  const merged = new Headers(headers || {})
  if (pythonAgentToken.value) {
    merged.set('Authorization', `Bearer ${pythonAgentToken.value}`)
  }
  return merged
}

async function createRemoteSession(): Promise<string> {
  if (!pythonAgentBaseUrl.value) {
    throw new Error('Python Agent 基础地址为空')
  }
  const sessionResp = await fetch(`${pythonAgentBaseUrl.value}/api/session/new`, {
    method: 'POST',
    headers: withSidecarHeaders({ 'Content-Type': 'application/json' }),
    body: '{}',
  })

  if (!sessionResp.ok) {
    throw new Error(`创建远端会话失败: ${sessionResp.status}`)
  }

  const sessionJson = (await sessionResp.json()) as { session_id?: string }
  if (!sessionJson.session_id) {
    throw new Error('远端会话返回为空')
  }
  return sessionJson.session_id
}

async function bindRemoteSessionForLocal(localSessionId: string) {
  if (!pythonAgentReady.value) return

  let remoteId = remoteSessionMap.value[localSessionId]
  if (!remoteId) {
    remoteId = await createRemoteSession()
    remoteSessionMap.value[localSessionId] = remoteId
    saveRemoteSessionMap()
  }

  pythonSessionId.value = remoteId

  const modelId = sessionStore.getSelectedModel(localSessionId) || selectedModelId.value
  if (modelId) {
    await ensureRemoteModelReady(modelId)
  }

  await ensureRemoteDatasourceBound(localSessionId)
}

function escapeCsvCell(value: unknown): string {
  if (value === null || value === undefined) return ''
  const s = String(value)
  if (s.includes('"') || s.includes(',') || s.includes('\n') || s.includes('\r')) {
    return `"${s.replace(/"/g, '""')}"`
  }
  return s
}

function chartPayloadToCsv(payload: ChartPayload): string {
  const columns = payload.columns.map((c) => c.name)
  const header = columns.map((c) => escapeCsvCell(c)).join(',')
  const lines = payload.rows.map((row) => columns.map((col) => escapeCsvCell(row[col])).join(','))
  return [header, ...lines].join('\n')
}

function dataFingerprint(payload: ChartPayload): string {
  const cols = payload.columns.map((c) => `${c.name}:${c.dtype}`).join('|')
  return `${dataStore.activeDatasetId}|${payload.total_rows}|${payload.rows.length}|${cols}`
}

async function ensureRemoteDatasourceBound(localSessionId: string) {
  if (!pythonAgentReady.value) return
  const remoteId = remoteSessionMap.value[localSessionId]
  if (!remoteId) return

  const wantDataset = selectedDatasources.value.includes('dataset') && !!dataStore.payload
  const wantUpload = selectedDatasources.value.includes('upload') && !!uploadedFileInfo.value
  if (!wantDataset && !wantUpload) return

  const datasetFp = wantDataset ? dataFingerprint(dataStore.payload!) : ''
  const uploadFp = uploadedFileInfo.value ? `${uploadedFileInfo.value.name}:${uploadedFileInfo.value.size}` : ''
  const selectedFp = [...selectedDatasources.value].sort().join(',')
  const combinedFp = `ds:${datasetFp}|up:${uploadFp}|sel:${selectedFp}`

  if (remoteDataSyncMap.value[localSessionId] === combinedFp) return

  // 1) 先上传主数据源（无 append）
  if (wantDataset) {
    const csv = chartPayloadToCsv(dataStore.payload!)
    const fileNameBase = dataStore.datasets.find((d) => d.id === dataStore.activeDatasetId)?.name || 'dataset_snapshot'
    const csvFile = new File([csv], `${fileNameBase}.csv`, { type: 'text/csv' })
    const fd = new FormData()
    fd.append('file', csvFile)
    const resp = await fetch(`${pythonAgentBaseUrl.value}/api/session/${remoteId}/upload`, {
      method: 'POST',
      headers: withSidecarHeaders(),
      body: fd,
    })
    if (!resp.ok) {
      const body = await resp.text()
      throw new Error(`远端数据上传失败(${resp.status}): ${body}`)
    }
  }

  // 2) 若还选择了手动上传文件，则追加合并
  if (wantUpload) {
    const fd = new FormData()
    fd.append('file', uploadedFileInfo.value!.file)
    const append = wantDataset ? '?append=true' : ''
    const resp = await fetch(`${pythonAgentBaseUrl.value}/api/session/${remoteId}/upload${append}`, {
      method: 'POST',
      headers: withSidecarHeaders(),
      body: fd,
    })
    if (!resp.ok) {
      const body = await resp.text()
      throw new Error(`手动上传文件同步失败(${resp.status}): ${body}`)
    }
  }

  remoteDataSyncMap.value[localSessionId] = combinedFp
  saveRemoteDataSyncMap()
}

function setDatasourceSelected(source: DatasourceChoice, checked: boolean) {
  const exists = selectedDatasources.value.includes(source)
  if (checked && !exists) {
    selectedDatasources.value.push(source)
  } else if (!checked && exists) {
    selectedDatasources.value = selectedDatasources.value.filter((x) => x !== source)
  }
}

async function syncRemoteSessionModel(modelId: string) {
  if (!pythonAgentBaseUrl.value || !pythonSessionId.value || !modelId) return

  const cfg = modelConfigs.value.find((m) => m.id === modelId)
  const provider = (cfg?.provider || modelId).trim()
  if (!provider) {
    throw new Error('模型 provider 为空')
  }

  const response = await fetch(`${pythonAgentBaseUrl.value}/api/session/${pythonSessionId.value}/model`, {
    method: 'POST',
    headers: withSidecarHeaders({ 'Content-Type': 'application/json' }),
    body: JSON.stringify({ provider }),
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`同步模型失败: ${response.status} ${body}`)
  }
}

async function syncModelConfigToRemote(modelId: string) {
  if (!pythonAgentBaseUrl.value || !modelId) return

  const cfg = modelConfigs.value.find((m) => m.id === modelId)
  if (!cfg) {
    throw new Error('未找到模型配置')
  }
  if (!cfg.apiKey.trim()) {
    throw new Error(`${cfg.displayName} 的 API Key 为空`)
  }
  if (!cfg.baseUrl.trim()) {
    throw new Error(`${cfg.displayName} 的 API Base URL 为空`)
  }
  if (!cfg.model.trim()) {
    throw new Error(`${cfg.displayName} 的 Model ID 为空`)
  }

  const provider = (cfg.provider || cfg.id).trim()
  const response = await fetch(`${pythonAgentBaseUrl.value}/api/models/sync`, {
    method: 'POST',
    headers: withSidecarHeaders({ 'Content-Type': 'application/json' }),
    body: JSON.stringify({
      provider,
      api_key: cfg.apiKey,
      base_url: cfg.baseUrl,
      model: cfg.model,
      enabled: cfg.enabled,
      is_custom: cfg.isCustom,
      context_window: cfg.contextWindow,
      max_output_tokens: cfg.maxOutputTokens,
      enable_thinking: cfg.enableThinking,
    }),
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`同步模型配置失败: ${response.status} ${body}`)
  }

  const json = (await response.json()) as { provider?: string }
  const remoteProvider = (json.provider || provider).trim()
  if (remoteProvider && cfg.provider !== remoteProvider) {
    cfg.provider = remoteProvider
  }
}

async function ensureRemoteModelReady(modelId: string) {
  await syncModelConfigToRemote(modelId)
  await syncRemoteSessionModel(modelId)
}

async function readSseResponse(
  response: Response,
  onEvent: (event: AiEvent & { message?: string; [key: string]: unknown }) => void
) {
  if (!response.body) {
    throw new Error('SSE 响应没有 body')
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder('utf-8')
  let buffer = ''

  while (true) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })

    while (true) {
      const separatorIndex = buffer.indexOf('\n\n')
      if (separatorIndex < 0) break

      const block = buffer.slice(0, separatorIndex).trim()
      buffer = buffer.slice(separatorIndex + 2)

      if (!block) continue

      const dataLines = block
        .split('\n')
        .filter((line) => line.startsWith('data:'))
        .map((line) => line.replace(/^data:\s?/, ''))

      if (dataLines.length === 0) continue

      const raw = dataLines.join('\n').trim()
      if (!raw) continue

      try {
        onEvent(JSON.parse(raw))
      } catch (error) {
        console.warn('[AIAnalysis] failed to parse SSE event:', raw, error)
      }
    }
  }
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

  if (pythonAgentReady.value) {
    void syncModelConfigToRemote(configId)
      .then(() => {
        ElMessage.success(`已保存并同步 ${cfg.displayName} 配置`)
      })
      .catch((error) => {
        ElMessage.warning(`本地已保存，侧车同步失败: ${error instanceof Error ? error.message : '未知错误'}`)
      })
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

  if (!pythonAgentBaseUrl.value || !pythonSessionId.value) {
    await bootstrapPythonAgent()
    if (sessionStore.currentSessionId) {
      await bindRemoteSessionForLocal(sessionStore.currentSessionId)
    }
  }

  if (!pythonAgentBaseUrl.value || !pythonSessionId.value) {
    ElMessage.warning('Python Agent 尚未就绪')
    return
  }

  try {
    await ensureRemoteModelReady(selectedModelId.value)
    await ensureRemoteDatasourceBound(sessionStore.currentSessionId)
  } catch (error) {
    console.error('[AIAnalysis] preflight sync failed:', error)
    ElMessage.warning(`数据预检查失败: ${error instanceof Error ? error.message : '未知错误'}`)
  }

  // 添加用户消息
  if (!options?.suppressUserMessage) {
    const userDisplayMessage = command ? `/${command} ${message}`.trim() : message
    sessionStore.addMessage('user', userDisplayMessage)
  }

  isStreaming.value = true
  try {
    const assistantMsg = sessionStore.addMessage('assistant', '', 'text_delta')
    streamAbortController.value = new AbortController()

    const response = await fetch(`${pythonAgentBaseUrl.value}/api/session/${pythonSessionId.value}/chat`, {
      method: 'POST',
      headers: withSidecarHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify({
        message,
        command,
        excel_tables: options?.excelTables || [],
        excel_filename: options?.excelFilename || '',
        report_title: options?.reportTitle || '',
        report_sections: options?.reportSections || [],
        ppt_title: options?.pptTitle || '',
        ppt_slides: options?.pptSlides || [],
        dashboard_name: options?.dashboardName || '',
        dashboard_widgets: options?.dashboardWidgets || [],
      }),
      signal: streamAbortController.value.signal,
    })

    if (!response.ok) {
      throw new Error(`聊天请求失败: ${response.status}`)
    }

    await readSseResponse(response, (event) => {
      if (event.type === 'text_delta' && typeof event.content === 'string') {
        sessionStore.appendToMessage(assistantMsg.id, event.content)
      } else if (event.type === 'text' && typeof event.content === 'string') {
        if (!sessionStore.currentSession?.messages.find((m) => m.id === assistantMsg.id)?.content) {
          sessionStore.appendToMessage(assistantMsg.id, event.content)
        }
        sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
      } else if (event.type === 'reasoning' && typeof event.content === 'string') {
        sessionStore.addMessage('assistant', event.content, 'thinking')
      } else if (event.type === 'tool_start') {
        sessionStore.addMessage('assistant', '', 'tool_start', {
          toolName: typeof event.tool === 'string' ? event.tool : '工具调用',
          display: typeof event.display === 'string' ? event.display : '执行中',
        })
      } else if (event.type === 'tool_result') {
        const resultText = typeof event.content === 'string' ? event.content : JSON.stringify(event)
        sessionStore.addMessage('assistant', resultText, 'text', {
          display: '工具执行结果',
        })
      } else if (event.type === 'excel_outline') {
        const ev = event as Record<string, unknown>
        const markdown = typeof ev.markdown === 'string' ? ev.markdown : '已生成导出计划。'
        const tables = Array.isArray(ev.tables) ? (ev.tables.filter((x) => typeof x === 'string') as string[]) : ['*']
        const filename = typeof ev.filename === 'string' ? ev.filename : ''
        sessionStore.addMessage('assistant', markdown, 'outline', {
          outlineType: 'excel',
          tables,
          filename,
        })
      } else if (event.type === 'report_outline') {
        const ev = event as Record<string, unknown>
        const markdown = typeof ev.markdown === 'string' ? ev.markdown : '已生成报告大纲。'
        const title = typeof ev.title === 'string' ? ev.title : '分析报告'
        const sections = Array.isArray(ev.sections) ? (ev.sections as Record<string, any>[]) : []
        sessionStore.addMessage('assistant', markdown, 'outline', {
          outlineType: 'report',
          title,
          sections,
        })
      } else if (event.type === 'ppt_outline') {
        const ev = event as Record<string, unknown>
        const markdown = typeof ev.markdown === 'string' ? ev.markdown : '已生成 PPT 大纲。'
        const title = typeof ev.title === 'string' ? ev.title : 'PPT'
        const slides = Array.isArray(ev.slides) ? (ev.slides as Record<string, any>[]) : []
        sessionStore.addMessage('assistant', markdown, 'outline', {
          outlineType: 'ppt',
          title,
          slides,
        })
      } else if (event.type === 'dashboard_outline') {
        const ev = event as Record<string, unknown>
        const markdown = typeof ev.markdown === 'string' ? ev.markdown : '已生成看板大纲。'
        const name = typeof ev.name === 'string' ? ev.name : 'Dashboard'
        const widgets = Array.isArray(ev.widgets) ? (ev.widgets as Record<string, any>[]) : []
        sessionStore.addMessage('assistant', markdown, 'outline', {
          outlineType: 'dashboard',
          name,
          widgets,
        })
      } else if (event.type === 'code_block' && typeof event.content === 'string') {
        sessionStore.addMessage('assistant', event.content, 'code_block')
      } else if (event.type === 'chart_ref' && typeof event.chart_id === 'string') {
        void fetch(`${pythonAgentBaseUrl.value}/api/chart/${event.chart_id}`, {
          headers: withSidecarHeaders(),
        })
          .then((r) => {
            if (!r.ok) return Promise.reject(new Error(`图表拉取失败: ${r.status}`))
            return r.text()
          })
          .then((html) => {
            sessionStore.addMessage('assistant', html, 'chart_html', { chartId: event.chart_id })
          })
          .catch((error) => {
            sessionStore.addMessage('assistant', String(error), 'error', { error: String(error) })
          })
      } else if (event.type === 'usage') {
        const inputTokens = Number((event as Record<string, unknown>).prompt_tokens || 0)
        const outputTokens = Number((event as Record<string, unknown>).completion_tokens || 0)
        const sessionTotalInput = Number((event as Record<string, unknown>).session_total_input || 0)
        const sessionTotalOutput = Number((event as Record<string, unknown>).session_total_output || 0)
        const contextWindow = Number((event as Record<string, unknown>).context_window || 0)
        const maxOutputTokens = Number((event as Record<string, unknown>).max_output_tokens || 0)

        tokenStats.value = {
          inputTokens,
          outputTokens,
          sessionTotalInput,
          sessionTotalOutput,
          contextWindow,
          maxOutputTokens,
        }
      } else if (event.type === 'stopped') {
        sessionStore.addMessage('system', '已停止当前生成', 'text')
        sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
      } else if (event.type === 'error') {
        sessionStore.updateMessage(assistantMsg.id, {
          type: 'error',
          metadata: { error: String(event.message || '未知错误') },
        })
      } else if (event.type === 'done') {
        sessionStore.updateMessage(assistantMsg.id, { type: 'text' })
      }
    })
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      sessionStore.addMessage('system', '请求已取消', 'text')
    } else {
      ElMessage.error(`发送失败: ${err instanceof Error ? err.message : '未知错误'}`)
    }
  } finally {
    streamAbortController.value = null
    isStreaming.value = false
  }
}

function handleCommand(cmd: string) {
  const commands: Record<string, string> = {
    analyze: '请分析这个数据集的主要特征和趋势。',
    visualize: '请根据数据生成合适的图表。',
    export: '请将分析结果导出为报告。',
    clean: '请帮我清洗这个数据集。',
  }

  const message = commands[cmd] || `执行命令: ${cmd}`
  handleSendMessage(message)
}

function handleSendWithCommand(payload: { command: string; message: string }) {
  const msg = payload.message?.trim() || '请基于当前数据执行该命令并给出结果。'
  void handleSendMessage(msg, payload.command)
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
  if (payload.action === 'cancel') {
    sessionStore.addMessage('system', '已取消本次导出计划', 'text')
    return
  }

  if (payload.action === 'revise') {
    const reviseMap = {
      excel: 'excel_revise',
      report: 'report_revise',
      ppt: 'ppt_revise',
      dashboard: 'dashboard_revise',
    } as const

    const currentJsonMap = {
      excel: `[CURRENT_EXCEL_JSON] ${JSON.stringify({
        tables: payload.tables || ['*'],
        filename: payload.filename || '',
      })}`,
      report: `[CURRENT_REPORT_JSON] ${JSON.stringify({
        title: payload.title || '分析报告',
        sections: payload.sections || [],
      })}`,
      ppt: `[CURRENT_SLIDES_JSON] ${JSON.stringify({
        title: payload.title || 'PPT',
        slides: payload.slides || [],
      })}`,
      dashboard: `[CURRENT_DASHBOARD_JSON] ${JSON.stringify({
        name: payload.name || 'Dashboard',
        widgets: payload.widgets || [],
      })}`,
    } as const

    const reviseInstructionMap = {
      excel: '请根据当前 Excel 导出方案进行修改。',
      report: '请根据当前报告大纲进行修改。',
      ppt: '请根据当前 PPT 大纲进行修改。',
      dashboard: '请根据当前看板大纲进行修改。',
    } as const

    const reviseMessage = `${reviseInstructionMap[payload.outlineType]}\n\n${currentJsonMap[payload.outlineType]}`
    void handleSendMessage(reviseMessage, reviseMap[payload.outlineType])
    return
  }

  if (payload.outlineType === 'excel') {
    void handleSendMessage('确认导出', 'excel_confirm', {
      suppressUserMessage: true,
      excelTables: payload.tables || ['*'],
      excelFilename: payload.filename || '',
    })
    return
  }

  if (payload.outlineType === 'report') {
    void handleSendMessage('确认生成报告', 'report_confirm', {
      suppressUserMessage: true,
      reportTitle: payload.title || '分析报告',
      reportSections: payload.sections || [],
    })
    return
  }

  if (payload.outlineType === 'ppt') {
    void handleSendMessage('确认生成PPT', 'ppt_confirm', {
      suppressUserMessage: true,
      pptTitle: payload.title || 'PPT',
      pptSlides: payload.slides || [],
    })
    return
  }

  void handleSendMessage('确认生成看板', 'dashboard_confirm', {
    suppressUserMessage: true,
    dashboardName: payload.name || 'Dashboard',
    dashboardWidgets: payload.widgets || [],
  })
}

async function handleUploadFile(file: File) {
  if (!pythonAgentBaseUrl.value || !pythonSessionId.value) {
    ElMessage.warning('请先启动 AI 分析服务并建立会话')
    return
  }
  try {
    const formData = new FormData()
    formData.append('file', file)
    const resp = await fetch(
      `${pythonAgentBaseUrl.value}/api/session/${pythonSessionId.value}/upload`,
      { method: 'POST', headers: withSidecarHeaders(), body: formData }
    )
    if (!resp.ok) {
      const body = await resp.text()
      throw new Error(`上传失败(${resp.status}): ${body}`)
    }
    // 标记当前 session 的数据同步状态已失效（强制下次重新上传当前数据集）
    const localSessionId = sessionStore.currentSessionId
    if (localSessionId) {
      delete remoteDataSyncMap.value[localSessionId]
      saveRemoteDataSyncMap()
    }
    uploadedFileInfo.value = { name: file.name, size: file.size, file }
    setDatasourceSelected('upload', true)
    ElMessage.success(`文件「${file.name}」已上传至当前会话`)
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '文件上传失败')
  }
}

async function handleStopStream() {
  if (pythonAgentBaseUrl.value && pythonSessionId.value) {
    try {
      await fetch(`${pythonAgentBaseUrl.value}/api/session/${pythonSessionId.value}/stop`, {
        method: 'POST',
        headers: withSidecarHeaders({ 'Content-Type': 'application/json' }),
        body: '{}',
      })
    } catch (error) {
      console.warn('[AIAnalysis] stop stream failed:', error)
    }
  }

  streamAbortController.value?.abort()
  streamAbortController.value = null
  isStreaming.value = false
  ElMessage.info('已停止生成')
}

function handleClearChat() {
  sessionStore.clearSessionHistory()
}
</script>

<template>
  <div class="ai-analysis-container">
    <!-- 左侧：会话侧边栏 + 模型选择 -->
    <aside :class="['ai-sidebar', { collapsed: sidebarCollapsed }]">
      <div v-if="sidebarCollapsed" class="sidebar-collapsed-only" title="展开" @click="sidebarCollapsed = !sidebarCollapsed">
        ›
      </div>

      <div v-else class="sidebar-header">
        <div class="sidebar-header-title">AI 分析</div>
        <el-button link class="sidebar-collapse-btn" @click="sidebarCollapsed = !sidebarCollapsed">
          <el-icon>
            <component :is="ArrowLeft" />
          </el-icon>
        </el-button>
      </div>

      <div v-show="!sidebarCollapsed" class="sidebar-body">
        <!-- 数据源卡片 -->
        <el-card class="panel-card datasource-panel-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <div class="panel-title">数据源</div>
              <el-button link class="panel-collapse-btn" @click="datasourceCollapsed = !datasourceCollapsed">
                <el-icon><component :is="datasourceCollapsed ? ArrowDown : ArrowUp" /></el-icon>
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

        <el-card class="panel-card model-panel-card" shadow="never">
        <template #header>
          <div class="panel-header">
            <div class="panel-title">模型配置</div>
            <el-button link class="panel-collapse-btn" @click="modelConfigCollapsed = !modelConfigCollapsed">
              <el-icon>
                <component :is="modelConfigCollapsed ? ArrowDown : ArrowUp" />
              </el-icon>
            </el-button>
          </div>
        </template>

          <div v-show="!modelConfigCollapsed" class="model-panel-body">
            <!-- 模型选择 -->
            <div class="model-label">模型</div>
            <div class="model-select-row">
              <div :class="['model-cards-container', { single: enabledModels.length <= 1 }]">
                <div
                  v-for="m in enabledModels"
                  :key="m.id"
                  :class="['model-card-button', { active: selectedModelId === m.id }]"
                  @click="selectModel(m.id)"
                >
                  <div class="model-card-name">{{ m.displayName }}</div>
                  <div class="model-card-badge">✓</div>
                </div>
              </div>
              <el-button class="model-settings-btn" circle @click="settingsVisible = true">
                <el-icon><Setting /></el-icon>
              </el-button>
            </div>

            <!-- 选中模型信息 -->
            <el-alert
              v-if="!selectedModel"
              type="warning"
              :closable="false"
              title="请先配置并启用一个模型"
              class="model-alert"
            />
            <el-descriptions v-else :column="1" size="small" border class="model-info-table">
              <el-descriptions-item label="模型名称">{{ selectedModel.displayName }}</el-descriptions-item>
              <el-descriptions-item label="Model ID">{{ selectedModel.model }}</el-descriptions-item>
              <el-descriptions-item label="Base URL">{{ selectedModel.baseUrl }}</el-descriptions-item>
              <el-descriptions-item label="上下文窗口">{{ selectedModel.contextWindow ?? '-' }}</el-descriptions-item>
              <el-descriptions-item label="最大输出">{{ selectedModel.maxOutputTokens ?? '-' }}</el-descriptions-item>
              <el-descriptions-item label="思考模式">{{ selectedModel.enableThinking ? '启用' : '关闭' }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>

        <!-- Python 服务控制卡片 -->
        <el-card class="panel-card python-service-panel-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <div class="panel-title">Python 服务</div>
              <el-button link class="panel-collapse-btn" @click="pythonServiceCollapsed = !pythonServiceCollapsed">
                <el-icon>
                  <component :is="pythonServiceCollapsed ? ArrowDown : ArrowUp" />
                </el-icon>
              </el-button>
            </div>
          </template>

          <div v-show="!pythonServiceCollapsed" class="python-service-panel-body">
            <!-- 服务状态 - 紧凑显示 -->
            <div class="service-status-compact">
              <div class="status-line">
                <span class="status-label">状态:</span>
                <el-tag :type="pythonAgentStatus?.running ? 'success' : 'info'" size="small">
                  {{ pythonAgentStatus?.running ? '运行' : '停止' }}
                </el-tag>
                <span v-if="pythonAgentStatus?.running" class="status-info">
                  <span class="divider">|</span>
                  <span class="label">端口:</span>
                  <span class="value">{{ pythonAgentStatus?.port }}</span>
                </span>
                <span v-if="pythonAgentStatus?.running && pythonAgentStatus?.pid" class="status-info">
                  <span class="divider">|</span>
                  <span class="label">PID:</span>
                  <span class="value">{{ pythonAgentStatus?.pid }}</span>
                </span>
              </div>
              <div v-if="pythonAgentStatus?.base_url" class="status-line">
                <span class="label">地址:</span>
                <span class="value">{{ pythonAgentStatus?.base_url }}</span>
              </div>
              <div v-if="pythonAgentStatus?.app_dir" class="status-line status-line-secondary">
                <span class="label">目录:</span>
                <span class="value" :title="pythonAgentStatus?.app_dir">{{ pythonAgentStatus?.app_dir.split('/').pop() || pythonAgentStatus?.app_dir }}</span>
              </div>
            </div>

            <!-- 控制按钮 - 紧凑排列 -->
            <div class="service-controls-compact">
              <el-button
                :loading="pythonAgentLoading"
                :disabled="pythonAgentStatus?.running"
                type="primary"
                size="small"
                @click="startPythonAgent"
              >
                启动
              </el-button>
              <el-button
                :loading="pythonAgentLoading"
                :disabled="!pythonAgentStatus?.running"
                type="danger"
                size="small"
                @click="stopPythonAgent"
              >
                停止
              </el-button>
              <el-button
                :loading="pythonAgentLoading"
                type="info"
                size="small"
                @click="checkPythonAgentHealth"
              >
                检测
              </el-button>
            </div>
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
          :api-base-url="pythonAgentBaseUrl"
          :api-token="pythonAgentToken"
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

/* Python Service Panel - Compact */
.python-service-panel-body {
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.service-status-compact {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 6px;
  background: var(--el-fill-color);
  border-radius: 4px;
  border-left: 3px solid var(--el-color-info);
}

.status-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  flex-wrap: wrap;
  line-height: 1.4;
}

.status-line-secondary {
  color: var(--el-text-color-secondary);
}

.status-info {
  display: flex;
  align-items: center;
  gap: 4px;
}

.divider {
  color: var(--el-border-color);
  margin: 0 2px;
}

.status-label {
  font-weight: 500;
  color: var(--el-text-color-regular);
  white-space: nowrap;
}

.label {
  font-weight: 500;
  color: var(--el-text-color-regular);
  white-space: nowrap;
}

.value {
  color: var(--el-text-color-primary);
  font-family: 'Courier New', monospace;
  word-break: break-word;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.service-controls-compact {
  display: flex;
  gap: 4px;
  flex-wrap: nowrap;

  :deep(.el-button) {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    padding: 4px 8px;
  }

  :deep(.el-button span) {
    font-size: 12px;
  }
}
</style>