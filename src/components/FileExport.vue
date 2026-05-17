<!-- src/components/FileExport.vue -->
<!--
  文件导出组件 - 提供 Excel/PPT/Report/Dashboard 导出功能
  
  功能：
  - Excel 导出（自定义文件名）
  - PPT 生成（标题、配色、幻灯片数）
  - 报告生成（标题、章节数）
  - Dashboard 生成（名称、配色、组件数）
  - 文件路径复制功能
-->

<template>
  <div class="file-export-panel">
    <div class="panel-header">
      <h3>📁 文件导出</h3>
    </div>

    <!-- 导出类型选择 -->
    <el-tabs v-model="exportType" class="export-tabs">
      <!-- Excel 导出 -->
      <el-tab-pane label="Excel" name="excel">
        <el-form :model="excelForm" label-width="80px" size="small">
          <el-form-item label="文件名">
            <el-input
              v-model="excelForm.filename"
              placeholder="sales_report.xlsx"
              clearable
            />
          </el-form-item>
          <el-form-item label="数据表">
            <el-select
              v-model="excelForm.tables"
              multiple
              placeholder="选择数据表"
              style="width: 100%"
            >
              <el-option
                v-for="table in availableTables"
                :key="table"
                :label="table"
                :value="table"
              />
            </el-select>
          </el-form-item>
          <el-button
            type="primary"
            :loading="isExporting"
            @click="handleExportExcel"
            style="width: 100%"
          >
            📊 导出 Excel
          </el-button>
        </el-form>
      </el-tab-pane>

      <!-- PPT 生成 -->
      <el-tab-pane label="PPT" name="ppt">
        <el-form :model="pptForm" label-width="80px" size="small">
          <el-form-item label="标题">
            <el-input
              v-model="pptForm.title"
              placeholder="销售分析报告"
              clearable
            />
          </el-form-item>
          <el-form-item label="配色方案">
            <el-select v-model="pptForm.colorScheme" style="width: 100%">
              <el-option label="麦肯锡蓝" value="mckinsey" />
              <el-option label="BCG绿" value="bcg" />
              <el-option label="Bain橙" value="bain" />
              <el-option label="EY紫" value="ey" />
            </el-select>
          </el-form-item>
          <el-form-item label="幻灯片数">
            <el-slider
              v-model="pptForm.slideCount"
              :min="3"
              :max="15"
              :step="1"
              show-input
            />
          </el-form-item>
          <el-button
            type="primary"
            :loading="isExporting"
            @click="handleGeneratePPT"
            style="width: 100%"
          >
            📽️ 生成 PPT
          </el-button>
        </el-form>
      </el-tab-pane>

      <!-- 报告生成 -->
      <el-tab-pane label="报告" name="report">
        <el-form :model="reportForm" label-width="80px" size="small">
          <el-form-item label="标题">
            <el-input
              v-model="reportForm.title"
              placeholder="数据分析报告"
              clearable
            />
          </el-form-item>
          <el-form-item label="章节数">
            <el-slider
              v-model="reportForm.sectionCount"
              :min="2"
              :max="10"
              :step="1"
              show-input
            />
          </el-form-item>
          <el-button
            type="primary"
            :loading="isExporting"
            @click="handleGenerateReport"
            style="width: 100%"
          >
            📄 生成报告
          </el-button>
        </el-form>
      </el-tab-pane>

      <!-- Dashboard 生成 -->
      <el-tab-pane label="Dashboard" name="dashboard">
        <el-form :model="dashboardForm" label-width="80px" size="small">
          <el-form-item label="名称">
            <el-input
              v-model="dashboardForm.name"
              placeholder="销售看板"
              clearable
            />
          </el-form-item>
          <el-form-item label="配色方案">
            <el-select v-model="dashboardForm.colorScheme" style="width: 100%">
              <el-option label="麦肯锡蓝" value="mckinsey" />
              <el-option label="BCG绿" value="bcg" />
              <el-option label="Bain橙" value="bain" />
              <el-option label="EY紫" value="ey" />
            </el-select>
          </el-form-item>
          <el-form-item label="组件数">
            <el-slider
              v-model="dashboardForm.widgetCount"
              :min="4"
              :max="12"
              :step="2"
              show-input
            />
          </el-form-item>
          <el-button
            type="primary"
            :loading="isExporting"
            @click="handleGenerateDashboard"
            style="width: 100%"
          >
            📊 生成看板
          </el-button>
        </el-form>
      </el-tab-pane>
    </el-tabs>

    <!-- 导出结果展示 -->
    <div v-if="exportResult" class="export-result">
      <el-alert
        :title="exportResult.success ? '✅ 导出成功' : '❌ 导出失败'"
        :type="exportResult.success ? 'success' : 'error'"
        :closable="false"
        show-icon
      >
        <template #default>
          <p>{{ exportResult.message }}</p>
          <p v-if="exportResult.filePath" class="file-path">
            <el-link type="primary" @click="copyFilePath(exportResult.filePath)">
              📋 复制文件路径
            </el-link>
          </p>
        </template>
      </el-alert>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { useAgent } from '@/composables/useAgent'

// 使用 Agent composable
const { exportExcel, generatePPT, generateReport, generateDashboard } = useAgent()

// 导出类型
const exportType = ref('excel')

// 导出状态
const isExporting = ref(false)
const exportResult = ref<{
  success: boolean
  message: string
  filePath?: string
} | null>(null)

// 可用的数据表列表（从 store 或其他地方获取）
const availableTables = ref<string[]>(['main_data', 'summary'])

// Excel 表单
const excelForm = reactive({
  filename: '',
  tables: [] as string[],
})

// PPT 表单
const pptForm = reactive({
  title: '销售分析报告',
  colorScheme: 'mckinsey',
  slideCount: 5,
})

// 报告表单
const reportForm = reactive({
  title: '数据分析报告',
  sectionCount: 3,
})

// Dashboard 表单
const dashboardForm = reactive({
  name: '销售看板',
  colorScheme: 'mckinsey',
  widgetCount: 6,
})

/**
 * 导出 Excel
 */
async function handleExportExcel() {
  if (excelForm.tables.length === 0) {
    ElMessage.warning('请至少选择一个数据表')
    return
  }

  isExporting.value = true
  exportResult.value = null

  try {
    const result = await exportExcel(excelForm.tables, excelForm.filename || undefined)
    exportResult.value = {
      success: true,
      message: result.message,
      filePath: result.file_path,
    }
    ElMessage.success('Excel 导出成功')
  } catch (error) {
    exportResult.value = {
      success: false,
      message: String(error),
    }
    ElMessage.error('Excel 导出失败')
  } finally {
    isExporting.value = false
  }
}

/**
 * 生成 PPT
 */
async function handleGeneratePPT() {
  if (!pptForm.title.trim()) {
    ElMessage.warning('请输入 PPT 标题')
    return
  }

  isExporting.value = true
  exportResult.value = null

  try {
    const result = await generatePPT(pptForm.title, pptForm.colorScheme, pptForm.slideCount)
    exportResult.value = {
      success: true,
      message: result.message,
      filePath: result.file_path,
    }
    ElMessage.success('PPT 生成成功')
  } catch (error) {
    exportResult.value = {
      success: false,
      message: String(error),
    }
    ElMessage.error('PPT 生成失败')
  } finally {
    isExporting.value = false
  }
}

/**
 * 生成报告
 */
async function handleGenerateReport() {
  if (!reportForm.title.trim()) {
    ElMessage.warning('请输入报告标题')
    return
  }

  isExporting.value = true
  exportResult.value = null

  try {
    const result = await generateReport(reportForm.title, reportForm.sectionCount)
    exportResult.value = {
      success: true,
      message: result.message,
      filePath: result.file_path,
    }
    ElMessage.success('报告生成成功')
  } catch (error) {
    exportResult.value = {
      success: false,
      message: String(error),
    }
    ElMessage.error('报告生成失败')
  } finally {
    isExporting.value = false
  }
}

/**
 * 生成 Dashboard
 */
async function handleGenerateDashboard() {
  if (!dashboardForm.name.trim()) {
    ElMessage.warning('请输入看板名称')
    return
  }

  isExporting.value = true
  exportResult.value = null

  try {
    const result = await generateDashboard(dashboardForm.name, dashboardForm.colorScheme, dashboardForm.widgetCount)
    exportResult.value = {
      success: true,
      message: result.message,
      filePath: result.file_path,
    }
    ElMessage.success('Dashboard 生成成功')
  } catch (error) {
    exportResult.value = {
      success: false,
      message: String(error),
    }
    ElMessage.error('Dashboard 生成失败')
  } finally {
    isExporting.value = false
  }
}

/**
 * 复制文件路径到剪贴板
 */
async function copyFilePath(filePath: string) {
  try {
    await navigator.clipboard.writeText(filePath)
    ElMessage.success('文件路径已复制到剪贴板')
  } catch (error) {
    ElMessage.error('复制失败')
  }
}
</script>

<style scoped>
.file-export-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--el-bg-color);
  border-left: 1px solid var(--el-border-color-light);
}

.panel-header {
  padding: 16px;
  border-bottom: 1px solid var(--el-border-color-light);
}

.panel-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.export-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.export-tabs :deep(.el-tabs__content) {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.el-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.export-result {
  padding: 16px;
  border-top: 1px solid var(--el-border-color-light);
}

.file-path {
  margin-top: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.file-path .el-link {
  font-size: 12px;
}
</style>
