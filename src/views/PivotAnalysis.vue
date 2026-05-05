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
//   4. 可选：将透视结果转为图表（调用 BiChart）

import { computed, ref, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import BiChart from '../components/BiChart.vue'
import { ECHARTS_THEME_OPTIONS } from '../utils/echartsTheme'
import type { ChartPayload } from '../utils/chartAdapter'
import type { EChartsOption, BarSeriesOption } from 'echarts'

const dataStore = useDataStore()

// ─── 透视参数状态 ────────────────────────────────────────────────────────────

const rowCols = ref<string[]>([])
const colCols = ref<string[]>([])
const valueCols = ref<string[]>([])
const aggFunc = ref<'sum' | 'mean' | 'count' | 'min' | 'max'>('sum')

const loading = ref(false)
const pivotPayload = ref<ChartPayload | null>(null)
const configCollapsed = ref(false)

const configSpan = computed(() => (configCollapsed.value ? 1 : 7))
const contentSpan = computed(() => (configCollapsed.value ? 23 : 17))

// ─── 可选：将透视表渲染为柱状图 ──────────────────────────────────────────────
const showChart = ref(false)
const pivotChartOption = shallowRef<EChartsOption | null>(null)

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
  pivotChartOption.value = null
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('pivot_data', {
      rows: rowCols.value,
      columns: colCols.value,
      values: valueCols.value,
      agg: aggFunc.value,
    })
    if (result.ok && result.data) {
      pivotPayload.value = result.data
      // 如果开启图表模式，构建 ECharts option
      if (showChart.value && result.data.rows.length > 0) {
        buildPivotChart(result.data)
      }
    } else {
      ElMessage.error(result.error ?? '透视计算失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

// 简单地将透视结果的每个值字段作为 bar 系列
function buildPivotChart(payload: ChartPayload) {
  const labelCol = payload.columns[0]?.name ?? ''
  const numCols = payload.columns.slice(1)

  const series: BarSeriesOption[] = numCols.map((col) => ({
    name: col.name,
    type: 'bar' as const,
    data: payload.rows.map((r) => Number(r[col.name] ?? 0)),
  }))

  const option: EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: { trigger: 'axis' as const },
    legend: { bottom: 0 },
    xAxis: {
      type: 'category',
      data: payload.rows.map((r) => String(r[labelCol] ?? '')),
      axisLabel: { rotate: 30 },
    },
    yAxis: { type: 'value' },
    series,
  }

  pivotChartOption.value = option
}
</script>

<template>
  <div class="pivot-analysis-view">
    <el-row :gutter="24" style="height: 100%;">
      <!-- 左侧：控制面板 -->
      <el-col :span="configSpan">
        <el-card v-if="!configCollapsed" class="panel-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <span>透视参数</span>
              <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
            </div>
          </template>
          <el-form label-width="80px" label-position="left" size="small" :disabled="!dataStore.hasData">

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

            <el-form-item label="可视化">
              <el-switch v-model="showChart" active-text="开启图表" />
            </el-form-item>

            <el-form-item>
              <el-button type="primary" :loading="loading" @click="runPivot" style="width:100%">
                执行透视
              </el-button>
            </el-form-item>

            <el-text v-if="pivotPayload" size="small" type="info" style="display:block; margin-top:8px">
              透视结果：{{ pivotPayload.total_rows }} 行 × {{ pivotPayload.columns.length }} 列
            </el-text>
          </el-form>
        </el-card>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </el-col>

      <!-- 右侧：结果展示 -->
      <el-col :span="contentSpan" class="content-col">
        <!-- 透视图表（可选） -->
        <el-card v-if="showChart && pivotChartOption" class="panel-card" style="margin-bottom:16px" shadow="never">
          <template #header>
            <div class="chart-card-header">
              <span>透视图表</span>
              <el-select
                :model-value="dataStore.currentTheme"
                size="small"
                style="width: 130px"
                placeholder="图表主题"
                @update:model-value="dataStore.setTheme"
              >
                <el-option v-for="opt in ECHARTS_THEME_OPTIONS" :key="opt.value" :label="opt.label" :value="opt.value" />
              </el-select>
            </div>
          </template>
          <BiChart :option="pivotChartOption" :loading="loading" height="300px" />
        </el-card>

        <!-- 透视表格 -->
        <el-card class="panel-card pivot-table-card" :header="`透视表（${pivotPayload?.total_rows ?? 0} 行）`" shadow="never">
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
      </el-col>
    </el-row>
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

.chart-card-header {
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
  min-height: 0;
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
</style>
