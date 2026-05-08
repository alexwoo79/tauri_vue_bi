<script setup lang="ts">
// src/views/PivotAnalysis.vue
// 多维透视表操作面板 (Pivot Table Analysis Panel)
//
// 对应原 bi/app.py 的 "🔢 Pivot分析" 模式。
//
// 功能：
//   1. 选择行分组、列分组、值字段、聚合方式
//   2. 调用 pivot_data 后端命令
//   3. 渲染透视表（el-table）
//   4. 支持逆透视 melt（unpivot）

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import type { ChartPayload } from '../utils/chartAdapter'
import { useResize } from '../composables/useResize'
import { useDatasetActions } from '../composables/useDatasetActions'

const dataStore = useDataStore()

// ─── 透视参数状态 ────────────────────────────────────────────────────────────

const rowCols = ref<string[]>([])
const colCols = ref<string[]>([])
const valueCols = ref<string[]>([])
const aggFunc = ref<'sum' | 'mean' | 'count' | 'min' | 'max'>('sum')
const meltIdCols = ref<string[]>([])
const meltValueCols = ref<string[]>([])
const meltVarName = ref('variable')
const meltValueName = ref('value')

const loading = ref(false)
const pivotPayload = ref<ChartPayload | null>(null)
const configCollapsed = ref(false)
const childDatasetName = ref('')
const meltChildDatasetName = ref('')
const formSections = ref<string[]>(['pivot', 'melt'])

const { configWidth, startResize } = useResize(320, 600)
const { loadDatasets } = useDatasetActions()

// ─── 执行透视 ────────────────────────────────────────────────────────────────

async function runPivot() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先在"数据加载"页面加载数据')
    return
  }
  if (rowCols.value.length === 0) {
    ElMessage.warning('至少选择一个行分组字段')
    return
  }
  if (valueCols.value.length === 0) {
    ElMessage.warning('至少选择一个值字段')
    return
  }

  loading.value = true
  pivotPayload.value = null
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('pivot_data', {
      rows: rowCols.value,
      columns: colCols.value,
      values: valueCols.value,
      agg: aggFunc.value,
      saveAsDataset: false,
    })
    if (result.ok && result.data) {
      pivotPayload.value = result.data
    } else {
      ElMessage.error(result.error ?? '透视计算失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function savePivotAsDataset() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  if (rowCols.value.length === 0 || valueCols.value.length === 0) {
    ElMessage.warning('请先配置透视参数并执行透视')
    return
  }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('pivot_data', {
      rows: rowCols.value,
      columns: colCols.value,
      values: valueCols.value,
      agg: aggFunc.value,
      saveAsDataset: true,
      datasetName: childDatasetName.value.trim() || `透视子数据_${Date.now()}`,
    })
    if (result.ok && result.data) {
      pivotPayload.value = result.data
      await loadDatasets()
      ElMessage.success('透视子数据已保存到数据列表')
    } else {
      ElMessage.error(result.error ?? '保存透视子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function runMelt() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先在"数据加载"页面加载数据')
    return
  }

  loading.value = true
  pivotPayload.value = null
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('melt_data', {
      idVars: meltIdCols.value,
      valueVars: meltValueCols.value,
      varName: meltVarName.value.trim() || 'variable',
      valueName: meltValueName.value.trim() || 'value',
      saveAsDataset: false,
    })
    if (result.ok && result.data) {
      pivotPayload.value = result.data
    } else {
      ElMessage.error(result.error ?? '逆透视失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveMeltAsDataset() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('melt_data', {
      idVars: meltIdCols.value,
      valueVars: meltValueCols.value,
      varName: meltVarName.value.trim() || 'variable',
      valueName: meltValueName.value.trim() || 'value',
      saveAsDataset: true,
      datasetName: meltChildDatasetName.value.trim() || `逆透视子数据_${Date.now()}`,
    })
    if (result.ok && result.data) {
      pivotPayload.value = result.data
      await loadDatasets()
      ElMessage.success('逆透视子数据已保存到数据列表')
    } else {
      ElMessage.error(result.error ?? '保存逆透视子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

function resetPivotConfig() {
  rowCols.value = []
  colCols.value = []
  valueCols.value = []
  aggFunc.value = 'sum'

  meltIdCols.value = []
  meltValueCols.value = []
  meltVarName.value = 'variable'
  meltValueName.value = 'value'

  childDatasetName.value = ''
  meltChildDatasetName.value = ''
  pivotPayload.value = null
  formSections.value = ['pivot', 'melt']

  ElMessage.success('透视与逆透视参数已重置')
}
</script>

<template>
  <div class="pivot-analysis-view">
    <div class="layout-row">
      <!-- 左侧：控制面板 -->
      <div class="config-col"
        :style="configCollapsed ? { width: '28px', minWidth: '28px' } : { width: configWidth + 'px', minWidth: configWidth + 'px' }">
        <div v-if="!configCollapsed" class="config-scroll">
          <el-card class="panel-card" shadow="never">
            <template #header>
              <div class="panel-header">
                <span>透视参数</span>
                <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
              </div>
            </template>
            <el-form class="compact-form" label-width="70px" label-position="left" size="small"
              :disabled="!dataStore.hasData">

              <el-collapse v-model="formSections" class="param-collapse">
                <el-collapse-item name="pivot">
                  <template #title>
                    <div class="section-title-row">
                      <span class="section-label">透视Pivot</span>
                    </div>
                  </template>

                  <el-form-item label="行分组">
                    <el-select v-model="rowCols" multiple placeholder="选择行分组字段" style="width:100%">
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>

                  <el-form-item label="列分组">
                    <el-select v-model="colCols" multiple placeholder="（可选）" clearable style="width:100%">
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>

                  <el-form-item label="值字段">
                    <el-select v-model="valueCols" multiple placeholder="选择值字段" style="width:100%">
                      <el-option v-for="c in dataStore.numericColumns" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>

                  <el-form-item label="聚合方式">
                    <el-radio-group v-model="aggFunc">
                      <el-radio-button value="sum">求和</el-radio-button>
                      <el-radio-button value="mean">均值</el-radio-button>
                      <el-radio-button value="count">计数</el-radio-button>
                      <el-radio-button value="min">最小</el-radio-button>
                      <el-radio-button value="max">最大</el-radio-button>
                    </el-radio-group>
                  </el-form-item>

                  <el-form-item class="action-row action-row-inline">
                    <el-button type="primary" :loading="loading" @click="runPivot" style="width:120px">
                      执行透视
                    </el-button>
                    <el-button plain type="warning" :disabled="loading" @click="resetPivotConfig" style="width:88px">
                      重置
                    </el-button>
                  </el-form-item>

                  <el-form-item label="子数据名">
                    <el-input v-model="childDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="action-row">
                    <el-button type="success" :loading="loading" style="width:160px" @click="savePivotAsDataset">
                      保存透视结果到数据列表
                    </el-button>
                  </el-form-item>
                </el-collapse-item>

                <el-collapse-item name="melt">
                  <template #title>
                    <div class="section-title-row">
                      <span class="section-label">逆透视 Melt</span>
                    </div>
                  </template>

                  <el-form-item label="标识列">
                    <el-select v-model="meltIdCols" multiple clearable placeholder="可选：作为 id 维度保留" style="width:100%">
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>

                  <el-form-item label="值列">
                    <el-select v-model="meltValueCols" multiple clearable placeholder="为空时默认选择非标识列" style="width:100%">
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>

                  <el-form-item label="变量列名">
                    <el-input v-model="meltVarName" placeholder="默认 variable" />
                  </el-form-item>

                  <el-form-item label="数值列名">
                    <el-input v-model="meltValueName" placeholder="默认 value" />
                  </el-form-item>

                  <el-form-item class="action-row action-row-inline">
                    <el-button type="primary" :loading="loading" @click="runMelt" style="width:120px">
                      执行逆透视
                    </el-button>
                    <el-button plain type="warning" :disabled="loading" @click="resetPivotConfig" style="width:88px">
                      重置
                    </el-button>
                  </el-form-item>

                  <el-form-item label="子数据名">
                    <el-input v-model="meltChildDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="action-row">
                    <el-button type="success" :loading="loading" style="width:160px" @click="saveMeltAsDataset">
                      保存逆透视结果到数据列表
                    </el-button>
                  </el-form-item>
                </el-collapse-item>
              </el-collapse>

              <el-text v-if="pivotPayload" size="small" type="info" style="display:block; margin-top:8px">
                当前结果：{{ pivotPayload.total_rows }} 行 × {{ pivotPayload.columns.length }} 列
              </el-text>
            </el-form>
          </el-card>
        </div>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </div>
      <div v-if="!configCollapsed" class="resize-handle" @mousedown.prevent="startResize" />

      <!-- 右侧：结果展示 -->
      <div class="content-col">
        <!-- 透视表格 -->
        <el-card class="panel-card pivot-table-card" :header="`结果表（${pivotPayload?.total_rows ?? 0} 行）`" shadow="never">
          <div v-if="!dataStore.hasData" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="80" />
          </div>
          <div v-else-if="!pivotPayload" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="80" />
          </div>
          <el-table v-else :data="pivotPayload.rows" border stripe size="small" style="width:100%" height="100%">
            <el-table-column v-for="col in pivotPayload.columns" :key="col.name" :prop="col.name" :label="col.name"
              min-width="120" show-overflow-tooltip />
          </el-table>
        </el-card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pivot-analysis-view {
  height: 100%;
  overflow: hidden;
}

.panel-card {
  background: var(--el-bg-color-overlay);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.collapse-trigger {
  font-size: 24px;
  padding: 0;
  line-height: 1;
}

.panel-collapse-btn {
  font-size: 16px;
  padding: 0;
  line-height: 1;
  height: auto;
}

.section-title-row {
  margin: 2px 0 8px;
}

.section-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.param-collapse {
  border-top: none;
  border-bottom: none;
}

.param-collapse :deep(.el-collapse-item__header) {
  height: 34px;
  line-height: 34px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.param-collapse :deep(.el-collapse-item__wrap) {
  border-bottom: none;
}

.param-collapse :deep(.el-collapse-item__content) {
  padding-bottom: 4px;
}

.collapsed-handle {
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

.collapsed-handle:hover {
  color: var(--el-color-primary);
}

.layout-row {
  height: 100%;
  display: flex;
  overflow: hidden;
}

.resize-handle {
  width: 5px;
  min-width: 5px;
  flex: none;
  cursor: col-resize;
  margin: 0 4px;
  border-radius: 2px;
  background: transparent;
  transition: background 0.15s;
}

.resize-handle:hover,
.resize-handle:active {
  background: var(--el-color-primary-light-5);
}

.content-col {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.config-col {
  flex: none;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.config-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 2px;
}

.pivot-table-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.pivot-table-card :deep(.el-card__body) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 8px 12px;
}

.display-empty {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

:deep(.el-card__header) {
  padding: 8px 16px;
}

.compact-form :deep(.el-form-item) {
  margin-bottom: 10px;
}

.action-row :deep(.el-form-item__content) {
  justify-content: flex-end;
}

.action-row-inline :deep(.el-form-item__content) {
  display: flex;
  flex-wrap: nowrap;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
}

.compact-form :deep(.el-button) {
  height: 30px;
}
</style>
