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
import { useDataStore } from '../stores/dataStore'
import BiChart from '../components/BiChart.vue'
import { ECHARTS_THEME_OPTIONS } from '../utils/echartsTheme'
import { buildChartOption } from '../utils/chartAdapter'
import type { ChartPayload, ChartType } from '../utils/chartAdapter'

const dataStore = useDataStore()

// ─── 图表参数状态 ────────────────────────────────────────────────────────────

const chartType = ref<ChartType>('bar_chart')
const xCol = ref('')
const yCol = ref('')
const colorCol = ref('')
const sortBy = ref<'x' | 'y' | 'none'>('none')
const sortAsc = ref(true)
const topnMode = ref<'off' | 'top' | 'bottom'>('off')
const topnValue = ref(10)

const loading = ref(false)
const chartPayload = ref<ChartPayload | null>(null)
const configCollapsed = ref(false)

const configSpan = computed(() => (configCollapsed.value ? 1 : 7))
const contentSpan = computed(() => (configCollapsed.value ? 23 : 17))

// ─── 图表类型选项 ────────────────────────────────────────────────────────────

const chartTypeOptions: { label: string; value: ChartType }[] = [
  { label: '柱状图 (Bar)', value: 'bar_chart' },
  { label: '折线图 (Line)', value: 'line_chart' },
  { label: '散点图 (Scatter)', value: 'scatter_chart' },
  { label: '饼图 (Pie)', value: 'pie_chart' },
  { label: '热力图 (Heatmap)', value: 'heatmap_chart' },
  { label: '箱线图 (Boxplot)', value: 'boxplot_chart' },
  { label: '面积图 (Area)', value: 'area_chart' },
  { label: '直方图 (Histogram)', value: 'histogram_chart' },
  { label: '密度图 (Density)', value: 'density_chart' },
]

// ─── 计算图表 option ─────────────────────────────────────────────────────────

const chartOption = computed(() => {
  if (!chartPayload.value || !xCol.value || !yCol.value) return null
  return buildChartOption(chartPayload.value, {
    chartType: chartType.value,
    xCol: xCol.value,
    yCol: yCol.value,
    colorCol: colorCol.value || undefined,
  })
})

// ─── 自动初始化列选择 ────────────────────────────────────────────────────────

watch(
  () => dataStore.columnNames,
  (names) => {
    if (names.length > 0 && !xCol.value) xCol.value = names[0]
    if (names.length > 1 && !yCol.value) yCol.value = names[1]
  },
  { immediate: true }
)

// ─── 生成图表 ────────────────────────────────────────────────────────────────

async function generateChart() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先在"数据加载"页面加载数据')
    return
  }
  if (!xCol.value || !yCol.value) {
    ElMessage.warning('请选择 X 轴和 Y 轴字段')
    return
  }

  // 计算 topN 参数（正数 = TopN，负数 = BottomN，0 = 关闭）
  let topN = 0
  if (topnMode.value === 'top') topN = topnValue.value
  else if (topnMode.value === 'bottom') topN = -topnValue.value

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
      'fetch_chart_data',
      {
        xCol: xCol.value,
        yCol: yCol.value,
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
</script>

<template>
  <div class="chart-analysis-view">
    <el-row :gutter="24" style="height: 100%;">
      <!-- 左侧：控制面板 -->
      <el-col :span="configSpan">
        <el-card v-if="!configCollapsed" class="panel-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <span>图表参数</span>
              <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
            </div>
          </template>
          <el-form label-width="80px" label-position="left" size="small" :disabled="!dataStore.hasData">

            <el-form-item label="图表类型">
              <el-select v-model="chartType" style="width:100%">
                <el-option v-for="opt in chartTypeOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
              </el-select>
            </el-form-item>

            <el-form-item label="X 轴">
              <el-select v-model="xCol" style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="Y 轴">
              <el-select v-model="yCol" style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="颜色分组">
              <el-select v-model="colorCol" placeholder="（可选）" clearable style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
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

            <el-form-item>
              <el-button type="primary" :loading="loading" @click="generateChart" style="width:100%">
                生成图表
              </el-button>
            </el-form-item>

            <!-- 数据摘要 -->
            <el-text v-if="chartPayload" size="small" type="info" style="display:block; margin-top:8px">
              当前数据：{{ chartPayload.total_rows }} 行 × {{ chartPayload.columns.length }} 列
            </el-text>
          </el-form>
        </el-card>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </el-col>

      <!-- 右侧：图表显示区 -->
      <el-col :span="contentSpan" class="content-col">
        <el-card class="panel-card chart-card" shadow="never">
          <template #header>
            <div class="chart-card-header">
              <span>图表预览</span>
              <div class="chart-card-header-actions">
                <el-select
                  :model-value="dataStore.currentTheme"
                  size="small"
                  style="width: 130px"
                  placeholder="图表主题"
                  @update:model-value="dataStore.setTheme"
                >
                  <el-option v-for="opt in ECHARTS_THEME_OPTIONS" :key="opt.value" :label="opt.label" :value="opt.value" />
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
      </el-col>
    </el-row>
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

.content-col {
  height: 100%;
  display: flex;
  flex-direction: column;
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
</style>
