<script setup lang="ts">
// src/views/TimeAnalysis.vue
// 时间序列分析面板 (Time Series Analysis Panel)
//
// 功能：
//   1. 日期衍生列   — 从日期列生成 年/月/季/周几 新列
//   2. 时间聚合     — 按年/月/季聚合数值列
//   3. 移动平均     — N 期窗口移动平均
//   4. 同比/环比    — YoY / MoM 增长率计算

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import type { ChartPayload } from '../utils/chartAdapter'
import { getBusinessColumnLabel, getBusinessOptionLabel } from '../utils/businessColumnLabels'
import { useResize } from '../composables/useResize'
import { useDatasetActions } from '../composables/useDatasetActions'

const dataStore = useDataStore()
const { configWidth, startResize } = useResize(300, 560)
const { loadDatasets } = useDatasetActions()

// ─── 公共状态 ──────────────────────────────────────────────────────────────────
const loading = ref(false)
const configCollapsed = ref(false)
const activeTab = ref<'derive' | 'agg' | 'ma' | 'growth' | 'fill'>('derive')
const tabCards: Array<{ key: 'derive' | 'agg' | 'ma' | 'growth' | 'fill'; label: string }> = [
  { key: 'derive', label: '① 衍生列' },
  { key: 'agg', label: '② 聚合' },
  { key: 'ma', label: '③ 滚动统计' },
  { key: 'growth', label: '④ 增长/累计' },
  { key: 'fill', label: '⑤ 缺失补全' },
]

// ─── 日期衍生列参数 ────────────────────────────────────────────────────────────
const deriveCol = ref('')
const deriveParts = ref<string[]>(['year', 'month'])
const deriveSaveName = ref('')
const derivePayload = ref<ChartPayload | null>(null)

// ─── 时间聚合参数 ─────────────────────────────────────────────────────────────
const aggDateCol = ref('')
const aggGranularity = ref<'year' | 'month' | 'quarter' | 'week'>('month')
const aggValueCols = ref<string[]>([])
const aggFunc = ref<'sum' | 'mean' | 'count' | 'min' | 'max'>('sum')
const aggDatasetName = ref('')
const aggPayload = ref<ChartPayload | null>(null)

// ─── 移动平均参数 ─────────────────────────────────────────────────────────────
const maDateCol = ref('')
const maValueCol = ref('')
const maWindow = ref(3)
const maMinPeriods = ref(1)
const maStatFunc = ref<'mean' | 'sum' | 'max' | 'min' | 'std'>('mean')
const maDatasetName = ref('')
const maPayload = ref<ChartPayload | null>(null)

// ─── 同比/环比参数 ────────────────────────────────────────────────────────────
const grDateCol = ref('')
const grValueCols = ref<string[]>([])
const grAggFunc = ref<'sum' | 'mean' | 'count' | 'min' | 'max'>('sum')
const grGranularity = ref<'year' | 'month' | 'quarter' | 'week'>('month')
const grMode = ref<'yoy' | 'mom' | 'cum' | 'cum_yoy' | 'cum_mom'>('mom')
const grNormalize = ref<'none' | 'zscore' | 'base100'>('none')
const grAlignDepth = ref(1)
const growthDatasetName = ref('')
const grPayload = ref<ChartPayload | null>(null)

// ─── 缺失补全参数 ─────────────────────────────────────────────────────────────
const fillDateCol = ref('')
const fillValueCol = ref('')
const fillGranularity = ref<'year' | 'month' | 'quarter' | 'week'>('month')
const fillAggFunc = ref<'sum' | 'mean' | 'count' | 'min' | 'max'>('sum')
const fillMethod = ref<'zero' | 'mean' | 'linear' | 'ffill' | 'bfill'>('linear')
const fillDatasetName = ref('')
const fillPayload = ref<ChartPayload | null>(null)

// ─── 排序状态（公用于各结果表） ──────────────────────────────────────────────
const sortCol = ref('')
const sortAsc = ref(true)
const sortedRows = ref<Record<string, any>[]>([])
const saveLoading = ref(false)

// ─── 日期列 / 数值列 过滤 ───────────────────────────────────────────────────

/** 数据集中所有列 */
const allCols = computed(() => dataStore.columnNames ?? [])

/** 数值列 */
const numericCols = computed(() => dataStore.numericColumns ?? [])

// ─── 当前 Tab 对应的结果 payload ──────────────────────────────────────────────
const currentPayload = computed<ChartPayload | null>(() => {
  switch (activeTab.value) {
    case 'derive': return derivePayload.value
    case 'agg':    return aggPayload.value
    case 'ma':     return maPayload.value
    case 'growth': return grPayload.value
    case 'fill': return fillPayload.value
    default:       return null
  }
})

const displayRows = computed(() =>
  sortedRows.value.length > 0 ? sortedRows.value : currentPayload.value?.rows ?? []
)

// ─── 排序处理 ─────────────────────────────────────────────────────────────────
function handleTableSort(evt: { prop: string; order: string | null }) {
  const { prop, order } = evt
  if (!prop || !order) {
    sortCol.value = ''
    sortedRows.value = []
    return
  }
  sortCol.value = prop
  sortAsc.value = order === 'ascending'
  const rows = currentPayload.value?.rows ?? []
  sortedRows.value = [...rows].sort((a, b) => {
    const av = a[prop], bv = b[prop]
    if (av == null && bv == null) return 0
    if (av == null) return sortAsc.value ? 1 : -1
    if (bv == null) return sortAsc.value ? -1 : 1
    if (typeof av === 'number' && typeof bv === 'number')
      return sortAsc.value ? av - bv : bv - av
    return sortAsc.value
      ? String(av).localeCompare(String(bv))
      : String(bv).localeCompare(String(av))
  })
}

function clearSort() {
  sortCol.value = ''
  sortedRows.value = []
}

// 切换 tab 时清除排序
function onTabChange(tab: string) {
  clearSort()
  activeTab.value = tab as typeof activeTab.value
}

function setActiveTab(tab: 'derive' | 'agg' | 'ma' | 'growth' | 'fill') {
  if (activeTab.value === tab) return
  onTabChange(tab)
}

function growthModeLabel(mode: typeof grMode.value): string {
  const map: Record<typeof grMode.value, string> = {
    yoy: '同比',
    mom: '环比',
    cum: '累计值',
    cum_yoy: '累计同比',
    cum_mom: '累计环比',
  }
  return map[mode]
}

async function saveSorted() {
  if (!sortCol.value) {
    ElMessage.warning('请先点击列头排序后再保存')
    return
  }
  saveLoading.value = true
  try {
    const result: { ok: boolean; data?: any; error?: string } = await invoke('sort_and_save_dataset', {
      sortCol: sortCol.value,
      sortAsc: sortAsc.value,
      datasetName: null,
    })
    if (result.ok && result.data) {
      ElMessage.success(`已保存为新数据集：${result.data.name}`)
      await loadDatasets()
    } else {
      ElMessage.error(result.error ?? '保存失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    saveLoading.value = false
  }
}

// ─── 1. 日期衍生列 ────────────────────────────────────────────────────────────
async function runDerive() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!deriveCol.value)   { ElMessage.warning('请选择日期列'); return }
  if (!deriveParts.value.length) { ElMessage.warning('请至少选择一项衍生部分'); return }

  loading.value = true
  derivePayload.value = null
  clearSort()
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_derive_columns', {
        dateCol: deriveCol.value,
        parts: deriveParts.value,
        saveName: deriveSaveName.value.trim() || null,
      })
    if (result.ok && result.data) {
      derivePayload.value = result.data
      await loadDatasets()
      ElMessage.success('日期衍生列已添加，并已保存为新数据集')
    } else {
      ElMessage.error(result.error ?? '衍生列生成失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

// ─── 2. 时间聚合 ──────────────────────────────────────────────────────────────
async function runAgg() {
  if (!dataStore.hasData)    { ElMessage.warning('请先加载数据'); return }
  if (!aggDateCol.value)     { ElMessage.warning('请选择日期列'); return }
  if (!aggValueCols.value.length) { ElMessage.warning('请选择数值列'); return }

  loading.value = true
  aggPayload.value = null
  clearSort()
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_agg', {
        dateCol: aggDateCol.value,
        granularity: aggGranularity.value,
        valueCols: aggValueCols.value,
        aggFunc: aggFunc.value,
        saveAsDataset: false,
        datasetName: null,
      })
    if (result.ok && result.data) {
      aggPayload.value = result.data
      ElMessage.success(`时间聚合完成，共 ${result.data.total_rows} 个周期`)
    } else {
      ElMessage.error(result.error ?? '时间聚合失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveAggAsDataset() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!aggDateCol.value) { ElMessage.warning('请选择日期列'); return }
  if (!aggValueCols.value.length) { ElMessage.warning('请选择数值列'); return }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_agg', {
        dateCol: aggDateCol.value,
        granularity: aggGranularity.value,
        valueCols: aggValueCols.value,
        aggFunc: aggFunc.value,
        saveAsDataset: true,
        datasetName: aggDatasetName.value.trim() || null,
      })
    if (result.ok && result.data) {
      aggPayload.value = result.data
      await loadDatasets()
      ElMessage.success('聚合结果已保存到数据列表')
      aggDatasetName.value = ''
    } else {
      ElMessage.error(result.error ?? '保存聚合子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

// ─── 3. 移动平均 ──────────────────────────────────────────────────────────────
async function runMA() {
  if (!dataStore.hasData)  { ElMessage.warning('请先加载数据'); return }
  if (!maDateCol.value)    { ElMessage.warning('请选择日期列'); return }
  if (!maValueCol.value)   { ElMessage.warning('请选择数值列'); return }
  if (maWindow.value < 1)  { ElMessage.warning('窗口大小至少为 1'); return }

  loading.value = true
  maPayload.value = null
  clearSort()
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_rolling_avg', {
        dateCol: maDateCol.value,
        valueCol: maValueCol.value,
        window: maWindow.value,
        minPeriods: maMinPeriods.value,
        statFunc: maStatFunc.value,
        saveAsDataset: false,
        datasetName: null,
      })
    if (result.ok && result.data) {
      maPayload.value = result.data
      ElMessage.success(`移动平均（MA${maWindow.value}）计算完成`)
    } else {
      ElMessage.error(result.error ?? '移动平均计算失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveMAAsDataset() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!maDateCol.value) { ElMessage.warning('请选择日期列'); return }
  if (!maValueCol.value) { ElMessage.warning('请选择数值列'); return }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_rolling_avg', {
        dateCol: maDateCol.value,
        valueCol: maValueCol.value,
        window: maWindow.value,
        minPeriods: maMinPeriods.value,
        statFunc: maStatFunc.value,
        saveAsDataset: true,
        datasetName: maDatasetName.value.trim() || null,
      })
    if (result.ok && result.data) {
      maPayload.value = result.data
      await loadDatasets()
      ElMessage.success('移动平均结果已保存到数据列表')
      maDatasetName.value = ''
    } else {
      ElMessage.error(result.error ?? '保存移动平均子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

// ─── 4. 同比/环比 ─────────────────────────────────────────────────────────────
async function runGrowth() {
  if (!dataStore.hasData)  { ElMessage.warning('请先加载数据'); return }
  if (!grDateCol.value)    { ElMessage.warning('请选择日期列'); return }
  if (!grValueCols.value.length) { ElMessage.warning('请至少选择一个数值列'); return }

  loading.value = true
  grPayload.value = null
  clearSort()
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_growth_rate', {
        dateCol: grDateCol.value,
        valueCol: grValueCols.value[0] ?? '',
        valueCols: grValueCols.value,
        aggFunc: grAggFunc.value,
        granularity: grGranularity.value,
        mode: grMode.value,
        normalizeMethod: grNormalize.value,
        alignDepth: grAlignDepth.value,
        saveAsDataset: false,
        datasetName: null,
      })
    if (result.ok && result.data) {
      grPayload.value = result.data
      const label = growthModeLabel(grMode.value)
      ElMessage.success(`${label}计算完成（${grValueCols.value.length} 条序列）`)
    } else {
      ElMessage.error(result.error ?? '增长率计算失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveGrowthAsDataset() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!grDateCol.value) { ElMessage.warning('请选择日期列'); return }
  if (!grValueCols.value.length) { ElMessage.warning('请至少选择一个数值列'); return }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_growth_rate', {
        dateCol: grDateCol.value,
        valueCol: grValueCols.value[0] ?? '',
        valueCols: grValueCols.value,
        aggFunc: grAggFunc.value,
        granularity: grGranularity.value,
        mode: grMode.value,
        normalizeMethod: grNormalize.value,
        alignDepth: grAlignDepth.value,
        saveAsDataset: true,
        datasetName: growthDatasetName.value.trim() || null,
      })
    if (result.ok && result.data) {
      grPayload.value = result.data
      await loadDatasets()
      ElMessage.success('增长率结果已保存到数据列表')
      growthDatasetName.value = ''
    } else {
      ElMessage.error(result.error ?? '保存增长率子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function runFillMissing() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!fillDateCol.value) { ElMessage.warning('请选择日期列'); return }
  if (!fillValueCol.value) { ElMessage.warning('请选择数值列'); return }

  loading.value = true
  fillPayload.value = null
  clearSort()
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_fill_missing', {
        dateCol: fillDateCol.value,
        valueCol: fillValueCol.value,
        granularity: fillGranularity.value,
        aggFunc: fillAggFunc.value,
        fillMethod: fillMethod.value,
        saveAsDataset: false,
        datasetName: null,
      })
    if (result.ok && result.data) {
      fillPayload.value = result.data
      ElMessage.success('缺失补全完成')
    } else {
      ElMessage.error(result.error ?? '缺失补全失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveFillAsDataset() {
  if (!dataStore.hasData) { ElMessage.warning('请先加载数据'); return }
  if (!fillDateCol.value) { ElMessage.warning('请选择日期列'); return }
  if (!fillValueCol.value) { ElMessage.warning('请选择数值列'); return }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } =
      await invoke('time_fill_missing', {
        dateCol: fillDateCol.value,
        valueCol: fillValueCol.value,
        granularity: fillGranularity.value,
        aggFunc: fillAggFunc.value,
        fillMethod: fillMethod.value,
        saveAsDataset: true,
        datasetName: fillDatasetName.value.trim() || null,
      })
    if (result.ok && result.data) {
      fillPayload.value = result.data
      await loadDatasets()
      ElMessage.success('缺失补全结果已保存到数据列表')
      fillDatasetName.value = ''
    } else {
      ElMessage.error(result.error ?? '保存缺失补全子数据失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

</script>

<template>
  <div class="time-analysis-view">
    <div class="layout-row">
      <!-- ── 左侧控制面板 ───────────────────────────────────────────────── -->
      <div class="config-col"
        :style="configCollapsed
          ? { width: '28px', minWidth: '28px' }
          : { width: configWidth + 'px', minWidth: configWidth + 'px' }">

        <div v-if="!configCollapsed" class="config-scroll">
          <el-card class="panel-card" shadow="never">
            <template #header>
              <div class="panel-header">
                <span>时间分析参数</span>
                <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
              </div>
            </template>

            <div class="tab-card-grid" role="tablist" aria-label="时间分析功能切换">
              <button
                v-for="item in tabCards"
                :key="item.key"
                type="button"
                class="tab-card-btn"
                :class="{ active: activeTab === item.key }"
                @click="setActiveTab(item.key)"
              >
                {{ item.label }}
              </button>
            </div>

            <div class="tab-panel-wrap">
              <!-- ① 日期衍生列 -->
              <div v-if="activeTab === 'derive'" class="tab-panel">
                <el-form class="compact-form" label-width="72px" label-position="left" size="small"
                  :disabled="!dataStore.hasData">
                  <el-form-item label="日期列">
                    <el-select v-model="deriveCol" placeholder="选择日期列" clearable style="width:100%">
                      <el-option v-for="c in allCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="衍生部分">
                    <el-checkbox-group v-model="deriveParts" class="parts-group">
                      <el-checkbox label="year">年</el-checkbox>
                      <el-checkbox label="month">月</el-checkbox>
                      <el-checkbox label="quarter">季</el-checkbox>
                      <el-checkbox label="weekday">周几</el-checkbox>
                    </el-checkbox-group>
                  </el-form-item>
                  <el-form-item label="保存名称">
                    <el-input v-model="deriveSaveName" placeholder="留空自动命名" />
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" :loading="loading" :disabled="!dataStore.hasData"
                      style="width:100%" @click="runDerive">
                      生成衍生列
                    </el-button>
                  </el-form-item>
                  <el-alert class="tip-alert" type="info" :closable="false" show-icon style="margin-top:8px">
                    衍生列会追加到当前数据集并自动保存为新数据集。
                  </el-alert>
                </el-form>
              </div>

              <!-- ② 时间聚合 -->
              <div v-else-if="activeTab === 'agg'" class="tab-panel">
                <el-form class="compact-form" label-width="72px" label-position="left" size="small"
                  :disabled="!dataStore.hasData">
                  <el-form-item label="日期列">
                    <el-select v-model="aggDateCol" placeholder="选择日期列" clearable style="width:100%">
                      <el-option v-for="c in allCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="时间粒度">
                    <el-radio-group v-model="aggGranularity">
                      <el-radio-button value="week">周</el-radio-button>
                      <el-radio-button value="year">年</el-radio-button>
                      <el-radio-button value="quarter">季</el-radio-button>
                      <el-radio-button value="month">月</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="数值列">
                    <el-select v-model="aggValueCols" multiple placeholder="选择数值列" style="width:100%">
                      <el-option v-for="c in numericCols" :key="c" :label="c" :value="c" />
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
                  <el-form-item>
                    <el-button type="primary" :loading="loading" :disabled="!dataStore.hasData"
                      style="width:100%" @click="runAgg">
                      执行聚合
                    </el-button>
                  </el-form-item>
                  <el-divider content-position="left" class="save-divider">保存结果</el-divider>
                  <el-form-item label="子数据名" class="save-name-row">
                    <el-input v-model="aggDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="save-action-row">
                    <el-button type="success" :loading="loading" style="width:100%" @click="saveAggAsDataset">
                      保存聚合结果到数据列表
                    </el-button>
                  </el-form-item>
                </el-form>
              </div>

              <!-- ③ 移动平均 -->
              <div v-else-if="activeTab === 'ma'" class="tab-panel">
                <el-form class="compact-form" label-width="72px" label-position="left" size="small"
                  :disabled="!dataStore.hasData">
                  <el-form-item label="日期列">
                    <el-select v-model="maDateCol" placeholder="排序依据" clearable style="width:100%">
                      <el-option v-for="c in allCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="数值列">
                    <el-select v-model="maValueCol" placeholder="计算移动均值" clearable style="width:100%">
                      <el-option v-for="c in numericCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="窗口大小">
                    <el-input-number v-model="maWindow" :min="1" :max="999" style="width:110px" />
                    <el-text class="hint" size="small">期</el-text>
                  </el-form-item>
                  <el-form-item label="最小期数">
                    <el-input-number v-model="maMinPeriods" :min="1" :max="999" style="width:110px" />
                    <el-text class="hint" size="small">期才输出值</el-text>
                  </el-form-item>
                  <el-form-item label="统计方式">
                    <el-radio-group v-model="maStatFunc">
                      <el-radio-button value="mean">均值</el-radio-button>
                      <el-radio-button value="sum">求和</el-radio-button>
                      <el-radio-button value="max">最大</el-radio-button>
                      <el-radio-button value="min">最小</el-radio-button>
                      <el-radio-button value="std">标准差</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" :loading="loading" :disabled="!dataStore.hasData"
                      style="width:100%" @click="runMA">
                      执行滚动统计
                    </el-button>
                  </el-form-item>
                  <el-divider content-position="left" class="save-divider">保存结果</el-divider>
                  <el-form-item label="子数据名" class="save-name-row">
                    <el-input v-model="maDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="save-action-row">
                    <el-button type="success" :loading="loading" style="width:100%" @click="saveMAAsDataset">
                      保存移动平均结果到数据列表
                    </el-button>
                  </el-form-item>
                </el-form>
              </div>

              <!-- ④ 增长/累计 -->
              <div v-else-if="activeTab === 'growth'" class="tab-panel">
                <el-form class="compact-form" label-width="72px" label-position="left" size="small"
                  :disabled="!dataStore.hasData">
                  <el-form-item label="日期列">
                    <el-select v-model="grDateCol" placeholder="选择日期列" clearable style="width:100%">
                      <el-option v-for="c in allCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="数值列">
                    <el-select v-model="grValueCols" multiple placeholder="可多选：用于多序列对比" clearable style="width:100%">
                      <el-option v-for="c in numericCols" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="聚合方式">
                    <el-radio-group v-model="grAggFunc">
                      <el-radio-button value="sum">求和</el-radio-button>
                      <el-radio-button value="mean">均值</el-radio-button>
                      <el-radio-button value="count">计数</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="时间粒度">
                    <el-radio-group v-model="grGranularity">
                      <el-radio-button value="week">周</el-radio-button>
                      <el-radio-button value="year">年</el-radio-button>
                      <el-radio-button value="quarter">季</el-radio-button>
                      <el-radio-button value="month">月</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="比较类型">
                    <el-radio-group v-model="grMode">
                      <el-radio-button value="mom">环比</el-radio-button>
                      <el-radio-button value="yoy">同比</el-radio-button>
                      <el-radio-button value="cum">累计值</el-radio-button>
                      <el-radio-button value="cum_mom">累计环比</el-radio-button>
                      <el-radio-button value="cum_yoy">累计同比</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="对齐深度" v-if="grMode === 'yoy' || grMode === 'cum_yoy'">
                    <el-input-number v-model="grAlignDepth" :min="1" :max="5" style="width:120px" />
                    <el-text class="hint" size="small">年（1=去年，2=前年，最多近5年）</el-text>
                  </el-form-item>
                  <el-form-item label="标准化">
                    <el-radio-group v-model="grNormalize">
                      <el-radio-button value="none">不标准化</el-radio-button>
                      <el-radio-button value="zscore">Z-Score</el-radio-button>
                      <el-radio-button value="base100">基期100</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" :loading="loading" :disabled="!dataStore.hasData"
                      style="width:100%" @click="runGrowth">
                      计算增长/累计
                    </el-button>
                  </el-form-item>
                  <el-divider content-position="left" class="save-divider">保存结果</el-divider>
                  <el-form-item label="子数据名" class="save-name-row">
                    <el-input v-model="growthDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="save-action-row">
                    <el-button type="success" :loading="loading" style="width:100%" @click="saveGrowthAsDataset">
                      保存增长/累计结果到数据列表
                    </el-button>
                  </el-form-item>
                  <el-alert class="growth-alert" type="info" :closable="false" show-icon style="margin-top:8px">
                    同比：与上一年同期相比（月粒度 shift 12，季粒度 shift 4）。<br />
                    环比：与上一个统计期相比（shift 1）。<br />
                    对齐深度：同比/累计同比支持近 N 年对齐（今年 vs 去年/前年/...）。<br />
                    累计同比/累计环比：先累计后比较，适合年度累计完成率分析。<br />
                    多选数值列可做多序列同周期对比，标准化可用于跨量纲趋势比较。
                  </el-alert>
                </el-form>
              </div>
              <!-- ⑤ 缺失补全 -->
              <div v-else class="tab-panel">
                <el-form class="compact-form" label-width="72px" label-position="left" size="small"
                  :disabled="!dataStore.hasData">
                  <el-form-item label="日期列">
                    <el-select v-model="fillDateCol" placeholder="选择日期列" clearable style="width:100%">
                      <el-option v-for="c in allCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="数值列">
                    <el-select v-model="fillValueCol" placeholder="选择数值列" clearable style="width:100%">
                      <el-option v-for="c in numericCols" :key="c" :label="c" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="时间粒度">
                    <el-radio-group v-model="fillGranularity">
                      <el-radio-button value="week">周</el-radio-button>
                      <el-radio-button value="year">年</el-radio-button>
                      <el-radio-button value="quarter">季</el-radio-button>
                      <el-radio-button value="month">月</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="聚合方式">
                    <el-radio-group v-model="fillAggFunc">
                      <el-radio-button value="sum">求和</el-radio-button>
                      <el-radio-button value="mean">均值</el-radio-button>
                      <el-radio-button value="count">计数</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="补全方式">
                    <el-radio-group v-model="fillMethod">
                      <el-radio-button value="linear">线性插值</el-radio-button>
                      <el-radio-button value="ffill">前值填充</el-radio-button>
                      <el-radio-button value="bfill">后值填充</el-radio-button>
                      <el-radio-button value="mean">均值填充</el-radio-button>
                      <el-radio-button value="zero">0 填充</el-radio-button>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" :loading="loading" :disabled="!dataStore.hasData"
                      style="width:100%" @click="runFillMissing">
                      执行缺失补全
                    </el-button>
                  </el-form-item>
                  <el-divider content-position="left" class="save-divider">保存结果</el-divider>
                  <el-form-item label="子数据名" class="save-name-row">
                    <el-input v-model="fillDatasetName" placeholder="可选，留空自动命名" />
                  </el-form-item>
                  <el-form-item class="save-action-row">
                    <el-button type="success" :loading="loading" style="width:100%" @click="saveFillAsDataset">
                      保存补全结果到数据列表
                    </el-button>
                  </el-form-item>
                </el-form>
              </div>
            </div>
          </el-card>
        </div>

        <!-- 展开按钮 -->
        <div v-else class="collapsed-handle" @click="configCollapsed = false" title="展开">›</div>
      </div>

      <div v-if="!configCollapsed" class="resize-handle" @mousedown.prevent="startResize" />

      <!-- ── 右侧结果区 ──────────────────────────────────────────────────── -->
      <div class="result-col">
        <!-- 空状态 -->
        <div v-if="!currentPayload" class="empty-state">
          <el-empty description="请在左侧选择分析类型并执行计算" />
        </div>

        <template v-else>
          <!-- 结果信息栏 -->
          <div class="result-info-bar">
            <span class="result-stat">
              共 {{ currentPayload.total_rows }} 行 × {{ currentPayload.columns.length }} 列
            </span>
            <template v-if="sortCol">
              <el-tag size="small" type="info">
                已按「{{ sortCol }}」{{ sortAsc ? '升序' : '降序' }}排序
              </el-tag>
              <el-button size="small" type="primary" :loading="saveLoading" @click="saveSorted">
                💾 保存排序结果
              </el-button>
              <el-button size="small" @click="clearSort">✕ 清除排序</el-button>
            </template>
          </div>

          <!-- 结果表格 -->
          <el-table
            :data="displayRows"
            class="result-table"
            border
            stripe
            height="100%"
            @sort-change="handleTableSort"
          >
            <el-table-column
              v-for="col in currentPayload.columns"
              :key="col.name"
              :prop="col.name"
              :label="getBusinessColumnLabel(col.name)"
              sortable="custom"
              min-width="120"
              show-overflow-tooltip
            />
          </el-table>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.time-analysis-view {
  height: 100%;
  overflow: hidden;
}

.layout-row {
  height: 100%;
  display: flex;
  overflow: hidden;
}

/* ── 左侧控制面板 ── */
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
  overflow-x: hidden;
  padding-right: 2px;
}

.panel-card {
  background: var(--el-bg-color-overlay);
  height: 100%;
}

.panel-card :deep(.el-card__header) {
  height: 35px;
  min-height: 35px;
  padding-left: 16px;
  padding-right: 16px;
  padding-top: 0;
  padding-bottom: 0;
  display: flex;
  align-items: center;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 28px;
  width: 100%;
}

.panel-collapse-btn {
  margin-left: auto;
  padding: 0;
  font-size: 16px;
  line-height: 1;
  height: auto;
}

.tab-card-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  margin-bottom: 14px;
}

.tab-card-btn {
  appearance: none;
  box-sizing: border-box;
  margin: 0;
  border: 1px solid var(--el-border-color);
  background: var(--el-fill-color-blank);
  color: var(--el-text-color-regular);
  border-radius: 0;
  min-height: 40px;
  padding: 10px 8px;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.2;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
}

.tab-card-btn:hover {
  border-color: var(--el-color-primary-light-5);
  box-shadow: 0 0 0 1px var(--el-color-primary-light-8) inset;
}

.tab-card-btn.active {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
  box-shadow: 0 0 0 1px var(--el-color-primary) inset;
}

.tab-panel-wrap {
  min-height: 0;
}

.tab-panel {
  display: block;
}

.parts-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.hint {
  margin-left: 6px;
  color: var(--el-text-color-secondary);
}

.tip-alert {
  --el-alert-padding: 10px 12px;
}

.tip-alert :deep(.el-alert__content) {
  font-size: 12px;
  line-height: 1.45;
}

.tip-alert :deep(.el-alert__description) {
  font-size: 12px;
  line-height: 1.55;
}

.tip-alert :deep(.el-alert__icon) {
  font-size: 18px;
  margin-right: 8px;
}

.growth-alert {
  --el-alert-padding: 10px 12px;
}

.growth-alert :deep(.el-alert__content) {
  font-size: 12px;
  line-height: 1.45;
}

.growth-alert :deep(.el-alert__description) {
  font-size: 12px;
  line-height: 1.55;
}

.growth-alert :deep(.el-alert__icon) {
  font-size: 18px;
  margin-right: 8px;
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

/* ── 折叠时展开按钮 ── */
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

/* ── 右侧结果区 ── */
.result-col {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.result-info-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 4px;
  font-size: 13px;
  flex-shrink: 0;
}

.result-stat {
  color: var(--el-text-color-secondary);
}

.result-table {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.compact-form :deep(.el-form-item) {
  margin-bottom: 10px;
}

.compact-form :deep(.el-button) {
  height: 30px;
}

.save-divider {
  margin: 12px 0 10px;
}

.save-divider :deep(.el-divider__text) {
  padding: 0 8px;
  font-weight: 600;
}

.save-name-row,
.save-action-row {
  margin-bottom: 10px;
}
</style>
