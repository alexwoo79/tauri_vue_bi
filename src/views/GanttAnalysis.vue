<script setup lang="ts">
// src/views/GanttAnalysis.vue
// 甘特图进度与统计分析面板 (Gantt Chart Analysis Panel)
//
// 对应原 bi/pages/01_gantt.py（Streamlit 页面）的功能。
//
// 功能：
//   1. 配置甘特图字段（任务列、开始日期列、结束日期列、颜色分组列、里程碑列）
//   2. 调用 fetch_gantt_data 后端命令获取数据
//   3. 渲染 BiGanttChart 甘特图组件
//   4. 显示任务统计（总任务数、最早开始、最晚结束、平均工期）

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import BiGanttChart from '../components/BiGanttChart.vue'
import { ECHARTS_THEME_OPTIONS } from '../utils/echartsTheme'
import type { ChartPayload } from '../utils/chartAdapter'

const dataStore = useDataStore()

// ─── 甘特图字段配置 ───────────────────────────────────────────────────────────

const taskCol = ref('')
const startCol = ref('')
const endCol = ref('')
const projectCol = ref('')
const colorCol = ref('')
const milestoneCol = ref('')
const detailCol = ref('')

const loading = ref(false)
const ganttPayload = ref<ChartPayload | null>(null)
const configCollapsed = ref(false)

const configSpan = computed(() => (configCollapsed.value ? 1 : 7))
const contentSpan = computed(() => (configCollapsed.value ? 23 : 17))

const showTaskDetails = ref(true)
const showDuration = ref(true)
const sortByStart = ref(true)
const autoNumber = ref(true)
const granularity = ref<'day' | 'week' | 'month' | 'quarter' | 'year'>('month')
const barLabel = ref<'none' | 'name' | 'duration' | 'dates' | 'nameAndDuration' | 'detail'>('none')

// ─── 统计摘要 ────────────────────────────────────────────────────────────────

const stats = computed(() => {
  if (!ganttPayload.value || ganttPayload.value.rows.length === 0) return null
  const rows = ganttPayload.value.rows
  const starts = rows
    .map((r) => new Date(String(r[startCol.value] ?? '')).getTime())
    .filter((t) => !isNaN(t))
  const ends = rows
    .map((r) => new Date(String(r[endCol.value] ?? '')).getTime())
    .filter((t) => !isNaN(t))

  if (starts.length === 0 || ends.length === 0) return null

  const minStart = new Date(Math.min(...starts))
  const maxEnd = new Date(Math.max(...ends))
  const durations = rows.map((r) => {
    const s = new Date(String(r[startCol.value] ?? '')).getTime()
    const e = new Date(String(r[endCol.value] ?? '')).getTime()
    return isNaN(s) || isNaN(e) ? 0 : (e - s) / (1000 * 60 * 60 * 24)
  })
  const avgDuration = durations.reduce((a, b) => a + b, 0) / durations.length
  const totalDurationDays = Math.max(0, Math.round((maxEnd.getTime() - minStart.getTime()) / (1000 * 60 * 60 * 24)))

  return {
    total: rows.length,
    earliestStart: minStart.toLocaleDateString('zh-CN'),
    latestEnd: maxEnd.toLocaleDateString('zh-CN'),
    avgDurationDays: avgDuration.toFixed(1),
    totalDurationDays,
  }
})

// ─── 自动推断字段 ────────────────────────────────────────────────────────────

function autoInferFields() {
  const names = dataStore.columnNames
  if (names.length === 0) return

  // 推断任务列：包含 task/name/任务 的列
  taskCol.value =
    names.find((c) => /task|name|任务/i.test(c)) ?? names[0]

  // 推断开始日期列
  startCol.value =
    names.find((c) => /start|begin|开始/i.test(c)) ??
    dataStore.dateColumns[0] ??
    names[Math.min(1, names.length - 1)]

  // 推断结束日期列
  endCol.value =
    names.find((c) => /end|finish|结束/i.test(c)) ??
    dataStore.dateColumns[1] ??
    names[Math.min(2, names.length - 1)]

  // 推断颜色分组列
  // 推断项目列/颜色分组列
  projectCol.value =
    names.find((c) => /project|phase|group|分组|项目/i.test(c)) ?? ''
  colorCol.value = projectCol.value

  // 里程碑列不自动推断，由用户选择
  milestoneCol.value = ''
  detailCol.value = names.find((c) => /owner|desc|detail|remark|说明|备注|负责人/i.test(c)) ?? ''
}

// 监听数据变化时自动推断
import { watch } from 'vue'
watch(() => dataStore.columnNames, autoInferFields, { immediate: true })

// ─── 获取甘特图数据 ───────────────────────────────────────────────────────────

async function loadGanttData() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先在"数据加载"页面加载数据')
    return
  }
  if (!taskCol.value || !startCol.value || !endCol.value) {
    ElMessage.warning('请选择任务列、开始日期列和结束日期列')
    return
  }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
      'fetch_gantt_data',
      {
        taskCol: taskCol.value,
        startCol: startCol.value,
        endCol: endCol.value,
        projectCol: projectCol.value || null,
        colorCol: colorCol.value || null,
        milestoneCol: milestoneCol.value || null,
        detailCol: detailCol.value || null,
      }
    )
    if (result.ok && result.data) {
      ganttPayload.value = result.data
    } else {
      ElMessage.error(result.error ?? '甘特图数据获取失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="gantt-analysis-view">
    <el-row :gutter="24" style="height: 100%;">
      <!-- 左侧：配置面板 -->
      <el-col :span="configSpan" class="config-col">
        <div v-if="!configCollapsed" class="config-scroll">
        <el-card class="panel-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <span>甘特图配置</span>
              <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
            </div>
          </template>
          <el-form label-width="90px" label-position="left" size="small" :disabled="!dataStore.hasData">

            <el-form-item label="任务名称列">
              <el-select v-model="taskCol" style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="开始日期列">
              <el-select v-model="startCol" style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="结束日期列">
              <el-select v-model="endCol" style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="项目列">
              <el-select v-model="projectCol" placeholder="（可选）" clearable style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="颜色分组列">
              <el-select v-model="colorCol" placeholder="（可选）" clearable style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="里程碑列">
              <el-select v-model="milestoneCol" placeholder="（可选）" clearable style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-form-item label="详情列">
              <el-select v-model="detailCol" placeholder="tooltip 显示字段" clearable style="width:100%">
                <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>

            <el-divider content-position="left">显示选项</el-divider>

            <el-form-item label="时间粒度">
              <el-select v-model="granularity" style="width:100%">
                <el-option label="日" value="day" />
                <el-option label="周" value="week" />
                <el-option label="月" value="month" />
                <el-option label="季度" value="quarter" />
                <el-option label="年" value="year" />
              </el-select>
            </el-form-item>

            <el-form-item label="按开始排序">
              <el-switch v-model="sortByStart" />
            </el-form-item>

            <el-form-item label="自动编号">
              <el-switch v-model="autoNumber" />
            </el-form-item>

            <el-form-item label="显示时长">
              <el-switch v-model="showDuration" />
            </el-form-item>

            <el-form-item label="显示详情">
              <el-switch v-model="showTaskDetails" />
            </el-form-item>

            <el-form-item label="横道标签">
              <el-select v-model="barLabel" style="width:100%">
                <el-option label="不显示" value="none" />
                <el-option label="任务名" value="name" />
                <el-option label="天数" value="duration" />
                <el-option label="日期区间" value="dates" />
                <el-option label="名称+天数" value="nameAndDuration" />
                <el-option label="详情列" value="detail" />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-button type="primary" :loading="loading" @click="loadGanttData" style="width:100%">
                生成甘特图
              </el-button>
            </el-form-item>

            <!-- 统计摘要 -->
            <template v-if="stats">
              <el-divider content-position="left">统计摘要</el-divider>
              <el-descriptions :column="1" border size="small">
                <el-descriptions-item label="总任务数">{{ stats.total }}</el-descriptions-item>
                <el-descriptions-item label="最早开始">{{ stats.earliestStart }}</el-descriptions-item>
                <el-descriptions-item label="最晚结束">{{ stats.latestEnd }}</el-descriptions-item>
                <el-descriptions-item label="项目总用时">{{ stats.totalDurationDays }} 天</el-descriptions-item>
                <el-descriptions-item label="平均工期">{{ stats.avgDurationDays }} 天</el-descriptions-item>
              </el-descriptions>
            </template>
          </el-form>
        </el-card>
        </div>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </el-col>

      <!-- 右侧：甘特图 -->
      <el-col :span="contentSpan" class="content-col">
        <el-card class="panel-card gantt-card" shadow="never">
          <template #header>
            <div class="chart-card-header">
              <span>甘特图（横道图）</span>
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
          <div v-if="!dataStore.hasData" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="100" />
          </div>
          <div v-else-if="!ganttPayload" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="80" />
          </div>
          <BiGanttChart v-else :rows="ganttPayload.rows" :task-col="taskCol" :start-col="startCol" :end-col="endCol"
            :project-col="projectCol || undefined" :color-col="colorCol || undefined"
            :milestone-col="milestoneCol || undefined" :detail-col="detailCol || undefined" :options="{
              showTaskDetails,
              showDuration,
              sortByStart,
              autoNumber,
              granularity,
              barLabel,
            }" :loading="loading" height="100%" />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<style scoped>
.gantt-analysis-view {
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
}

.config-col {
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

.gantt-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.gantt-card :deep(.el-card__body) {
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
