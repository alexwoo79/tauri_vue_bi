<script setup lang="ts">
// src/views/ChartAnalysis.vue
// 基础图表分析与 TopN 控制面板 (Chart Analysis Panel)
//
// 对应原 bi/app.py 的 "📊 图表分析" 模式。
//
// 功能：
//   1. 图表类型选择（bar/line/scatter/pie/heatmap/boxplot/area/histogram/density）
//   2. X 轴、Y 轴、颜色分组列选择
//   3. 排序控制（按 X / 按 Y / 无）
//   4. TopN 过滤（TopN / BottomN / 关闭）
//   5. 调用 fetch_chart_data 后端命令
//   6. 渲染 BiChart 通用图表组件

import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import {
  DataAnalysis,
  TrendCharts,
  Connection,
  PieChart,
  Grid,
  Histogram,
} from '@element-plus/icons-vue'
import { useDataStore } from '../stores/dataStore'
import BiChart from '../components/BiChart.vue'
import { ECHARTS_THEME_OPTIONS } from '../utils/echartsTheme'
import { buildChartOption } from '../utils/chartAdapter'
import type { ChartPayload, ChartType } from '../utils/chartAdapter'
import { getBusinessOptionLabel } from '../utils/businessColumnLabels'
import { useResize } from '../composables/useResize'

const dataStore = useDataStore()

// ─── 图表参数状态 ────────────────────────────────────────────────────────────

const chartType = ref<ChartType>('bar_chart')
const xCol = ref('')
const yAxisCount = ref(1)
const yCols = ref<string[]>([''])
const yAxisSides = ref<Array<'left' | 'right'>>(['left'])
const colorCol = ref('')
const sortBy = ref<'x' | 'y' | 'none'>('none')
const sortAsc = ref(true)
const topnMode = ref<'off' | 'top' | 'bottom'>('off')
const topnValue = ref(10)
const swapXY = ref(false)

const loading = ref(false)
const chartPayload = ref<ChartPayload | null>(null)
const configCollapsed = ref(false)

const { configWidth, startResize } = useResize(320, 600)

// ─── 图表类型选项 ────────────────────────────────────────────────────────────

const chartTypeOptions: { label: string; value: ChartType; icon: any }[] = [
  { label: '柱状图', value: 'bar_chart', icon: Histogram },
  { label: '折线图', value: 'line_chart', icon: TrendCharts },
  { label: '散点图', value: 'scatter_chart', icon: Connection },
  { label: '饼图', value: 'pie_chart', icon: PieChart },
  { label: '热力图', value: 'heatmap_chart', icon: Grid },
  { label: '箱线图', value: 'boxplot_chart', icon: DataAnalysis },
  { label: '面积图', value: 'area_chart', icon: TrendCharts },
  { label: '直方图', value: 'histogram_chart', icon: Histogram },
  { label: '密度图', value: 'density_chart', icon: TrendCharts },
]

const activeChartTypeLabel = computed(
  () => chartTypeOptions.find((opt) => opt.value === chartType.value)?.label ?? ''
)

const activeYCols = computed(() =>
  yCols.value.slice(0, yAxisCount.value).filter((c) => !!c)
)
const activeYAxisSides = computed(() => yAxisSides.value.slice(0, yAxisCount.value))

const displayedXCol = computed(() => (swapXY.value ? (activeYCols.value[0] ?? '') : xCol.value))
const displayedYCols = computed(() => (swapXY.value ? (xCol.value ? [xCol.value] : []) : activeYCols.value))
const displayedYAxisSides = computed<Array<'left' | 'right'>>(() =>
  swapXY.value ? ['left'] : activeYAxisSides.value
)

// ─── 计算图表 option ─────────────────────────────────────────────────────────

const chartOption = computed(() => {
  if (!chartPayload.value || !displayedXCol.value || displayedYCols.value.length === 0) return null
  return buildChartOption(chartPayload.value, {
    chartType: chartType.value,
    xCol: displayedXCol.value,
    yCol: displayedYCols.value[0],
    yCols: displayedYCols.value,
    yAxisSides: displayedYAxisSides.value,
    swapAxes: swapXY.value,
    colorCol: colorCol.value || undefined,
  })
})

// ─── 自动初始化列选择 ────────────────────────────────────────────────────────

watch(
  () => dataStore.columnNames,
  (names) => {
    if (names.length > 0 && !xCol.value) xCol.value = names[0]
    if (names.length > 1 && !yCols.value[0]) yCols.value[0] = names[1]
  },
  { immediate: true }
)

watch(yAxisCount, (count) => {
  const safeCount = Math.min(8, Math.max(1, count))
  if (safeCount !== count) {
    yAxisCount.value = safeCount
    return
  }

  if (yCols.value.length < safeCount) {
    const candidates = dataStore.numericColumns
    while (yCols.value.length < safeCount) {
      const next = candidates[yCols.value.length] ?? ''
      yCols.value.push(next)
    }
  } else if (yCols.value.length > safeCount) {
    yCols.value = yCols.value.slice(0, safeCount)
  }

  if (yAxisSides.value.length < safeCount) {
    while (yAxisSides.value.length < safeCount) {
      yAxisSides.value.push(yAxisSides.value.length % 2 === 0 ? 'left' : 'right')
    }
  } else if (yAxisSides.value.length > safeCount) {
    yAxisSides.value = yAxisSides.value.slice(0, safeCount)
  }
})

const SWAP_INCOMPATIBLE_TYPES = new Set<ChartType>(['pie_chart', 'heatmap_chart', 'boxplot_chart'])

function ensureSwapCompatibleChartType(showToast = true) {
  if (!swapXY.value) return
  if (!SWAP_INCOMPATIBLE_TYPES.has(chartType.value)) return
  chartType.value = 'line_chart'
  if (showToast) {
    ElMessage.warning('当前图表类型不支持 X/Y 互换，已自动切换为折线图')
  }
}

watch(
  () => [swapXY.value, chartType.value],
  () => ensureSwapCompatibleChartType(true)
)

// ─── 生成图表 ────────────────────────────────────────────────────────────────

async function generateChart() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先在"数据加载"页面加载数据')
    return
  }
  if (!xCol.value || activeYCols.value.length === 0) {
    ElMessage.warning('请选择 X 轴和 Y 轴字段')
    return
  }

  ensureSwapCompatibleChartType(true)

  // 计算 topN 参数（正数 = TopN，负数 = BottomN，0 = 关闭）
  let topN = 0
  if (topnMode.value === 'top') topN = topnValue.value
  else if (topnMode.value === 'bottom') topN = -topnValue.value

  const requestXCol = swapXY.value ? activeYCols.value[0] : xCol.value
  const requestYCols = swapXY.value ? [xCol.value].filter(Boolean) : activeYCols.value
  if (!requestXCol || requestYCols.length === 0) {
    ElMessage.warning('互换后 X/Y 轴字段无效，请检查字段选择')
    return
  }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
      'fetch_chart_data',
      {
        xCol: requestXCol,
        yCols: requestYCols,
        colorCol: colorCol.value || null,
        sortBy: sortBy.value,
        sortAsc: sortAsc.value,
        topN,
      }
    )
    if (result.ok && result.data) {
      chartPayload.value = result.data
    } else {
      ElMessage.error(result.error ?? '数据获取失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

function swapAxes() {
  const firstY = yCols.value[0] ?? ''
  if (!xCol.value || !firstY) {
    ElMessage.warning('请先选择 X 轴和 Y1 列后再互换')
    return
  }
  const prevX = xCol.value
  xCol.value = firstY
  yCols.value[0] = prevX
  ElMessage.success('已互换 X 与 Y1')
}
</script>

<template>
  <div class="chart-analysis-view">
    <div class="layout-row">
      <!-- 左侧：控制面板 -->
      <div class="config-col"
        :style="configCollapsed ? { width: '28px', minWidth: '28px' } : { width: configWidth + 'px', minWidth: configWidth + 'px' }">
        <div v-if="!configCollapsed" class="config-scroll">
          <el-card class="panel-card" shadow="never">
            <template #header>
              <div class="panel-header">
                <span>图表参数</span>
                <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
              </div>
            </template>
            <el-form class="compact-form" label-width="70px" label-position="left" size="small"
              :disabled="!dataStore.hasData">

              <el-form-item label="图表类型">
                <div class="chart-type-grid">
                  <el-tooltip v-for="opt in chartTypeOptions" :key="opt.value" :content="opt.label" placement="top"
                    :show-after="120">
                    <el-button class="chart-type-btn" :class="{ 'is-active': chartType === opt.value }"
                      @click="chartType = opt.value">
                      <el-icon>
                        <component :is="opt.icon" />
                      </el-icon>
                    </el-button>
                  </el-tooltip>
                </div>
                <el-text class="chart-type-caption" size="small" type="info">当前：{{ activeChartTypeLabel }}</el-text>
              </el-form-item>

              <el-form-item label="X 轴">
                <el-select v-model="xCol" style="width:100%">
                  <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
                </el-select>
              </el-form-item>

              <el-form-item label="Y 轴个数">
                <el-input-number v-model="yAxisCount" :min="1" :max="8" />
              </el-form-item>

              <el-form-item v-for="idx in yAxisCount" :key="`y-col-${idx}`" :label="`Y${idx}`">
                <div class="y-col-config-row">
                  <el-select v-model="yCols[idx - 1]" style="width:100%" placeholder="选择数值列">
                    <el-option v-for="c in dataStore.numericColumns" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
                  </el-select>
                  <el-radio-group v-model="yAxisSides[idx - 1]" size="small">
                    <el-radio-button label="left">左轴</el-radio-button>
                    <el-radio-button label="right">右轴</el-radio-button>
                  </el-radio-group>
                </div>
              </el-form-item>

              <el-form-item label="轴向操作">
                <el-switch v-model="swapXY" active-text="请求时互换 X/Y" />
                <el-button text type="primary" @click="swapAxes">立即互换 X 与 Y1</el-button>
              </el-form-item>

              <el-form-item label="颜色分组">
                <el-select v-model="colorCol" placeholder="（可选）" clearable style="width:100%">
                  <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
                </el-select>
              </el-form-item>

              <el-divider content-position="left">排序</el-divider>
              <el-form-item label="排序依据">
                <el-radio-group v-model="sortBy">
                  <el-radio-button value="none">无</el-radio-button>
                  <el-radio-button value="x">按 X</el-radio-button>
                  <el-radio-button value="y">按 Y</el-radio-button>
                </el-radio-group>
              </el-form-item>
              <el-form-item label="排序方向" v-if="sortBy !== 'none'">
                <el-radio-group v-model="sortAsc">
                  <el-radio-button :value="true">升序</el-radio-button>
                  <el-radio-button :value="false">降序</el-radio-button>
                </el-radio-group>
              </el-form-item>

              <el-divider content-position="left">TopN 过滤</el-divider>
              <el-form-item label="模式">
                <el-radio-group v-model="topnMode">
                  <el-radio-button value="off">关闭</el-radio-button>
                  <el-radio-button value="top">TopN</el-radio-button>
                  <el-radio-button value="bottom">BottomN</el-radio-button>
                </el-radio-group>
              </el-form-item>
              <el-form-item label="N 值" v-if="topnMode !== 'off'">
                <el-input-number v-model="topnValue" :min="1" :max="10000" />
              </el-form-item>

              <el-form-item class="action-row">
                <el-button type="primary" :loading="loading" @click="generateChart" style="width:120px">
                  生成图表
                </el-button>
              </el-form-item>

              <!-- 数据摘要 -->
              <el-text v-if="chartPayload" size="small" type="info" style="display:block; margin-top:8px">
                当前数据：{{ chartPayload.total_rows }} 行 × {{ chartPayload.columns.length }} 列
              </el-text>
            </el-form>
          </el-card>
        </div>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </div>
      <div v-if="!configCollapsed" class="resize-handle" @mousedown.prevent="startResize" />

      <!-- 右侧：图表显示区 -->
      <div class="content-col">
        <el-card class="panel-card chart-card" shadow="never">
          <template #header>
            <div class="chart-card-header">
              <span>图表预览</span>
              <div class="chart-card-header-actions">
                <el-select :model-value="dataStore.currentTheme" size="small" style="width: 130px" placeholder="图表主题"
                  @update:model-value="dataStore.setTheme">
                  <el-option v-for="opt in ECHARTS_THEME_OPTIONS" :key="opt.value" :label="opt.label"
                    :value="opt.value" />
                </el-select>
                <el-tag v-if="chartPayload" size="small">
                  {{ chartType.replace('_chart', '').toUpperCase() }}
                </el-tag>
              </div>
            </div>
          </template>

          <div v-if="!dataStore.hasData" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="100" />
          </div>
          <BiChart v-else :option="chartOption" :loading="loading" height="100%" />
        </el-card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chart-analysis-view {
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

.chart-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.chart-card-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
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

.chart-type-grid {
  display: grid;
  grid-template-columns: repeat(4, 48px);
  grid-auto-rows: 34px;
  gap: 6px;
  width: max-content;
  max-width: 100%;
}

.chart-type-grid :deep(.el-tooltip__trigger) {
  display: block;
  width: 48px;
  height: 34px;
}

.chart-type-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100% !important;
  min-width: 0;
  padding: 0;
  margin: 0;
  box-sizing: border-box;
  border-radius: 0;
  border: 1px solid var(--el-border-color);
  background: var(--el-fill-color-blank);
  transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
}

.chart-type-btn :deep(.el-icon) {
  flex: 0 0 auto;
  font-size: 18px;
}

.chart-type-btn:hover {
  border-color: var(--el-color-primary-light-5);
  box-shadow: 0 0 0 1px var(--el-color-primary-light-8) inset;
}

.chart-type-btn.is-active {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
  box-shadow: 0 0 0 1px var(--el-color-primary) inset;
  background: var(--el-color-primary-light-9);
}

.chart-type-caption {
  display: block;
  margin-top: 6px;
}

.action-row :deep(.el-form-item__content) {
  justify-content: flex-end;
}

.y-col-config-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
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

.chart-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.chart-card :deep(.el-card__body) {
  flex: 1;
  min-height: 0;
  padding: 8px;
  display: flex;
  flex-direction: column;
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

.compact-form :deep(.el-button) {
  height: 30px;
}
</style>
