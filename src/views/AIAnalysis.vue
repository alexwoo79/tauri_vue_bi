<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Setting } from '@element-plus/icons-vue'

interface AiModelConfig {
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
})

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
</script>

<template>
  <div class="ai-analysis-view">
    <div class="ai-sidebar">
      <el-card class="panel-card" shadow="never">
        <template #header>
          <div class="panel-title">AI智能分析</div>
        </template>

        <div class="model-label">模型</div>
        <div class="model-select-row">
          <div class="model-cards-container">
            <div
              v-for="m in enabledModels"
              :key="m.id"
              :class="['model-card-button', { active: selectedModelId === m.id }]"
              @click="selectedModelId = m.id"
            >
              <div class="model-card-name">{{ m.displayName }}</div>
              <div class="model-card-badge">✓</div>
            </div>
          </div>
          <el-button class="model-settings-btn" circle @click="settingsVisible = true">
            <el-icon><Setting /></el-icon>
          </el-button>
        </div>

        <el-alert
          v-if="!selectedModel"
          type="warning"
          :closable="false"
          title="请先配置并启用一个模型"
        />
        <el-descriptions v-else :column="1" size="small" border class="model-info-table">
          <el-descriptions-item label="模型名称">{{ selectedModel.displayName }}</el-descriptions-item>
          <el-descriptions-item label="Model ID">{{ selectedModel.model }}</el-descriptions-item>
          <el-descriptions-item label="Base URL">{{ selectedModel.baseUrl }}</el-descriptions-item>
          <el-descriptions-item label="上下文窗口">{{ selectedModel.contextWindow ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="最大输出">{{ selectedModel.maxOutputTokens ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="思考模式">{{ selectedModel.enableThinking ? '启用' : '关闭' }}</el-descriptions-item>
        </el-descriptions>
      </el-card>
    </div>

    <div class="ai-main">
      <el-card class="panel-card" shadow="never">
        <template #header>
          <div class="panel-title">对话分析</div>
        </template>
        <div class="placeholder">
          <h3>AI 分析会话区</h3>
          <p>已选择模型后，可在此区域继续接入会话分析、MCP 工具调用和结果展示流程。</p>
        </div>
      </el-card>
    </div>

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
.ai-analysis-view {
  height: 100%;
  display: flex;
  gap: 12px;
}

.ai-sidebar {
  width: 340px;
  min-width: 300px;
  max-width: 380px;
}

.ai-main {
  flex: 1;
  min-width: 0;
}

.panel-card {
  height: 100%;
}

.panel-title {
  font-size: 15px;
  font-weight: 700;
}

.model-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--el-text-color-regular);
  margin-bottom: 8px;
}

.model-select-row {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.model-cards-container {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.model-card-button {
  height: 64px;
  padding: 12px 16px;
  border: 1.5px solid var(--el-border-color);
  border-radius: 6px;
  background-color: var(--el-fill-color-blank);
  cursor: pointer;
  transition: all 0.25s ease;
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: relative;
}

.model-card-button:hover {
  border-color: var(--el-color-primary-light-3);
  background-color: var(--el-color-primary-light-9);
}

.model-card-button.active {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
  box-shadow: 0 0 0 2px rgba(94, 124, 224, 0.1);
}

.model-card-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  flex: 1;
}

.model-card-button.active .model-card-badge {
  display: flex;
}

.model-card-badge {
  display: none;
  width: 20px;
  height: 20px;
  background-color: var(--el-color-primary);
  color: white;
  border-radius: 50%;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: bold;
  margin-left: 8px;
}

.model-settings-btn {
  width: 44px;
  height: 44px;
  font-size: 20px;
  border: 1px solid var(--el-border-color);
}

.model-info-table {
  margin-top: 4px;
}

.placeholder {
  min-height: 220px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: var(--el-text-color-regular);
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
  .ai-analysis-view {
    flex-direction: column;
  }

  .ai-sidebar {
    width: 100%;
    min-width: 0;
    max-width: none;
  }
}
</style>