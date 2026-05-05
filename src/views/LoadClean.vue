<script setup lang="ts">
// src/views/LoadClean.vue
// 数据加载与清洗面板 (Data Loading & Cleaning Panel)
//
// 对应原 bi/app.py 的 "⬇️ 清洗导出" 模式。
//
// 功能：
//   1. 文件选择（CSV / Excel）+ 加载参数（跳行、表头行）
//   2. 数据预览（前 100 行，el-table）
//   3. 清洗操作面板（依次执行）：
//      a. 列过滤          (column filter)
//      b. 行条件过滤      (row filter)
//      c. 填充缺失值      (fillna)
//      d. 去重            (dedup)
//      e. 去除前后空格    (trim)
//      f. 查找替换        (find & replace)
//      g. 类型转换        (type cast)
//   4. 预览清洗结果 + 导出（通过 Tauri 文件系统）

import { shallowRef, ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import type { ChartPayload } from '../utils/chartAdapter'

const dataStore = useDataStore()

// ─── 状态 ─────────────────────────────────────────────────────────────────────

const filePath = ref('')
const skipHead = ref(0)
const skipTail = ref(0)
const headerRow = ref(-1)
const loading = ref(false)

// 清洗参数
const filterCols = ref<string[]>([])
type RowFilterOp =
  | 'eq'
  | 'ne'
  | 'gt'
  | 'ge'
  | 'lt'
  | 'le'
  | 'contains'
  | 'not_contains'
  | 'starts_with'
  | 'ends_with'
  | 'is_null'
  | 'not_null'
const rowFilterCol = ref('')
const rowFilterOp = ref<RowFilterOp>('eq')
const rowFilterVal = ref('')
const fillnaCol = ref('')
const fillnaVal = ref('')
const dedupCols = ref<string[]>([])
const trimCols = ref<string[]>([])
const frCols = ref<string[]>([])
const findText = ref('')
const replaceText = ref('')
const useRegex = ref(false)
const typeCol = ref('')
const typeTarget = ref<'int' | 'float' | 'str' | 'datetime' | 'date'>('str')

// 清洗后预览数据
const cleanedPayload = shallowRef<ChartPayload | null>(null)
const cleanLoading = ref(false)
const configCollapsed = ref(false)
const configSpan = computed(() => (configCollapsed.value ? 1 : 8))
const contentSpan = computed(() => (configCollapsed.value ? 23 : 16))
const activeCleanSections = ref<string[]>([
  'columnFilter',
  'rowFilter',
  'fillna',
  'dedup',
  'trim',
  'findReplace',
  'typeCast',
])

const rowFilterOpOptions: { label: string; value: RowFilterOp }[] = [
  { label: '等于 (=)', value: 'eq' },
  { label: '不等于 (≠)', value: 'ne' },
  { label: '大于 (>)', value: 'gt' },
  { label: '大于等于 (>=)', value: 'ge' },
  { label: '小于 (<)', value: 'lt' },
  { label: '小于等于 (<=)', value: 'le' },
  { label: '包含', value: 'contains' },
  { label: '不包含', value: 'not_contains' },
  { label: '前缀匹配', value: 'starts_with' },
  { label: '后缀匹配', value: 'ends_with' },
  { label: '为空', value: 'is_null' },
  { label: '非空', value: 'not_null' },
]

// ─── 计算属性 ─────────────────────────────────────────────────────────────────

// 用于 el-table 的列定义（来自已加载的 payload）
const tableColumns = computed(
  () => (cleanedPayload.value ?? dataStore.payload)?.columns ?? []
)

// 预览的行（优先显示清洗后，否则显示原始）
const previewRows = computed(
  () => (cleanedPayload.value ?? dataStore.payload)?.rows ?? []
)

// ─── 文件选择 ────────────────────────────────────────────────────────────────

async function selectFile() {
  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: '数据文件', extensions: ['csv', 'xlsx', 'xls', 'xlsm'] }],
    })
    if (selected && typeof selected === 'string') {
      filePath.value = selected
    }
  } catch (e: any) {
    ElMessage.error(`文件选择失败: ${String(e)}`)
  }
}

// ─── 加载文件 ────────────────────────────────────────────────────────────────

async function loadFile() {
  if (!filePath.value) {
    ElMessage.warning('请先选择文件')
    return
  }
  loading.value = true
  cleanedPayload.value = null
  const perfLabel = '📊 数据加载性能'
  try {
    console.time(perfLabel)
    console.log('📥 开始 Tauri IPC 调用...')

    const t1 = performance.now()
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('load_file', {
      path: filePath.value,
      skipHead: skipHead.value,
      skipTail: skipTail.value,
      headerRow: headerRow.value,
    })
    const t2 = performance.now()
    console.log(`⏱️  Tauri IPC: ${(t2 - t1).toFixed(2)}ms`)

    if (result.ok && result.data) {
      console.log(`📦 数据包大小: ${result.data.total_rows} 行 × ${result.data.columns.length} 列`)
      console.log(`📝 预览行数: ${result.data.rows.length} 行`)

      const t3 = performance.now()
      dataStore.setPayload(result.data)
      const t4 = performance.now()
      console.log(`⚡ 状态更新: ${(t4 - t3).toFixed(2)}ms (shallowRef 优化)`)

      // 重置清洗参数
      filterCols.value = []
      rowFilterCol.value = ''
      rowFilterOp.value = 'eq'
      rowFilterVal.value = ''
      fillnaCol.value = ''
      dedupCols.value = []
      trimCols.value = []
      frCols.value = []
      typeCol.value = ''
      console.timeEnd(perfLabel)
      ElMessage.success(`✅ 数据加载成功，共 ${result.data.total_rows} 行（预览 ${result.data.rows.length} 行）`)
    } else {
      ElMessage.error(result.error ?? '加载失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

// ─── 应用清洗 ────────────────────────────────────────────────────────────────

async function applyClean() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  cleanLoading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('clean_data', {
      filterCols: filterCols.value,
      rowFilterCol: rowFilterCol.value,
      rowFilterOp: rowFilterOp.value,
      rowFilterVal: rowFilterVal.value,
      fillnaCol: fillnaCol.value,
      fillnaVal: fillnaVal.value,
      dedupCols: dedupCols.value,
      trimCols: trimCols.value,
      frCols: frCols.value,
      findText: findText.value,
      replaceText: replaceText.value,
      useRegex: useRegex.value,
      typeCol: typeCol.value,
      typeTarget: typeTarget.value,
    })
    if (result.ok && result.data) {
      cleanedPayload.value = result.data
      // Keep store metadata in sync so dtype tags and cross-page column types update.
      dataStore.setPayload(result.data)
      ElMessage.success(`清洗完成，共 ${result.data.total_rows} 行`)
    } else {
      ElMessage.error(result.error ?? '清洗失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    cleanLoading.value = false
  }
}

type CleanSection = 'columnFilter' | 'rowFilter' | 'fillna' | 'dedup' | 'trim' | 'findReplace' | 'typeCast'

function rowFilterNeedsValue(op: RowFilterOp) {
  return op !== 'is_null' && op !== 'not_null'
}

function buildSectionPayload(section: CleanSection) {
  return {
    filterCols: section === 'columnFilter' ? filterCols.value : [],
    rowFilterCol: section === 'rowFilter' ? rowFilterCol.value : '',
    rowFilterOp: section === 'rowFilter' ? rowFilterOp.value : 'eq',
    rowFilterVal: section === 'rowFilter' ? rowFilterVal.value : '',
    fillnaCol: section === 'fillna' ? fillnaCol.value : '',
    fillnaVal: section === 'fillna' ? fillnaVal.value : '',
    dedupCols: section === 'dedup' ? dedupCols.value : [],
    trimCols: section === 'trim' ? trimCols.value : [],
    frCols: section === 'findReplace' ? frCols.value : [],
    findText: section === 'findReplace' ? findText.value : '',
    replaceText: section === 'findReplace' ? replaceText.value : '',
    useRegex: section === 'findReplace' ? useRegex.value : false,
    typeCol: section === 'typeCast' ? typeCol.value : '',
    typeTarget: section === 'typeCast' ? typeTarget.value : 'str',
  }
}

async function applySectionClean(section: CleanSection) {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }

  if (section === 'fillna' && !fillnaCol.value) {
    ElMessage.warning('请选择填充缺失值的目标列')
    return
  }
  if (section === 'columnFilter' && filterCols.value.length === 0) {
    ElMessage.warning('请至少选择一列进行去除')
    return
  }
  if (section === 'rowFilter' && !rowFilterCol.value) {
    ElMessage.warning('请选择行条件过滤的目标列')
    return
  }
  if (section === 'rowFilter' && rowFilterNeedsValue(rowFilterOp.value) && !rowFilterVal.value) {
    ElMessage.warning('请输入行条件过滤值')
    return
  }
  if (section === 'typeCast' && !typeCol.value) {
    ElMessage.warning('请选择类型转换的目标列')
    return
  }
  if (section === 'findReplace' && !findText.value) {
    ElMessage.warning('请输入查找文本')
    return
  }

  cleanLoading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('clean_data', buildSectionPayload(section))
    if (result.ok && result.data) {
      cleanedPayload.value = result.data
      dataStore.setPayload(result.data)
      ElMessage.success('当前清洗项已应用')
    } else {
      ElMessage.error(result.error ?? '清洗失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    cleanLoading.value = false
  }
}

async function undoCleanStep() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  cleanLoading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('undo_clean')
    if (result.ok && result.data) {
      cleanedPayload.value = result.data
      dataStore.setPayload(result.data)
      ElMessage.success('已撤销一步清洗')
    } else {
      ElMessage.warning(result.error ?? '没有可撤销步骤')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    cleanLoading.value = false
  }
}

async function rollbackClean() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  cleanLoading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('rollback_clean')
    if (result.ok && result.data) {
      dataStore.setPayload(result.data)
      cleanedPayload.value = null
      ElMessage.success('已回退到加载时原始数据')
    } else {
      ElMessage.error(result.error ?? '回退失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    cleanLoading.value = false
  }
}

// ─── 重置清洗 ────────────────────────────────────────────────────────────────

function resetClean() {
  cleanedPayload.value = null
  filterCols.value = []
  rowFilterCol.value = ''
  rowFilterOp.value = 'eq'
  rowFilterVal.value = ''
  fillnaCol.value = ''
  fillnaVal.value = ''
  dedupCols.value = []
  trimCols.value = []
  frCols.value = []
  findText.value = ''
  replaceText.value = ''
  useRegex.value = false
  typeCol.value = ''
  typeTarget.value = 'str'
}

// ─── 导出文件 ────────────────────────────────────────────────────────────────

const saveLoading = ref(false)

async function exportFile(format: 'csv' | 'xlsx') {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  try {
    const savePath = await saveDialog({
      filters: format === 'xlsx'
        ? [{ name: 'Excel 工作表', extensions: ['xlsx'] }]
        : [{ name: 'CSV 文件', extensions: ['csv'] }],
      defaultPath: `export.${format}`,
    })
    if (!savePath) return

    saveLoading.value = true
    const result: { ok: boolean; data?: string; error?: string } = await invoke('save_file', {
      path: savePath,
    })
    if (result.ok) {
      ElMessage.success(`已导出到: ${result.data}`)
    } else {
      ElMessage.error(result.error ?? '导出失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    saveLoading.value = false
  }
}
</script>

<template>
  <div class="load-clean-view">
    <el-row :gutter="24" style="height: 100%;">
      <!-- 左侧：加载 + 清洗参数面板 -->
      <el-col :span="configSpan" class="config-col">
        <div v-if="!configCollapsed" class="config-scroll">
        <el-card class="panel-card config-unified-card" shadow="never">
          <template #header>
            <div class="panel-header">
              <span>数据处理</span>
              <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
            </div>
          </template>

          <!-- ① 数据加载 -->
          <div class="section-title-row">
            <span class="section-label">① 数据加载</span>
          </div>
          <el-form label-width="90px" label-position="left" size="small">
            <el-form-item label="选择文件">
              <el-input v-model="filePath" placeholder="点击右侧按钮选择文件" readonly>
                <template #append>
                  <el-button @click="selectFile">浏览…</el-button>
                </template>
              </el-input>
            </el-form-item>
            <el-form-item label="跳过开头行">
              <el-input-number v-model="skipHead" :min="0" :max="9999" />
            </el-form-item>
            <el-form-item label="跳过末尾行">
              <el-input-number v-model="skipTail" :min="0" :max="9999" />
            </el-form-item>
            <el-form-item label="表头行索引">
              <el-input-number v-model="headerRow" :min="-1" :max="9999" />
              <el-text class="hint" size="small">-1 = 首行为表头</el-text>
            </el-form-item>
            <el-form-item>
              <el-button class="action-btn load-btn" type="primary" :loading="loading" @click="loadFile">
                加载数据
              </el-button>
            </el-form-item>
          </el-form>

          <!-- ② 数据清洗 -->
          <el-divider class="section-divider" />
          <div class="section-title-row">
            <span class="section-label">② 数据清洗</span>
            <el-button text class="toggle-all-btn" @click="activeCleanSections = activeCleanSections.length ? [] : ['columnFilter', 'rowFilter', 'fillna', 'dedup', 'trim', 'findReplace', 'typeCast']">
              {{ activeCleanSections.length ? '全部折叠' : '全部展开' }}
            </el-button>
          </div>
          <el-form label-width="90px" label-position="left" size="small" :disabled="!dataStore.hasData">
            <el-collapse v-model="activeCleanSections" class="clean-collapse">
              <el-collapse-item title="去除列" name="columnFilter">
                <el-form-item label="去除列">
                  <el-select v-model="filterCols" multiple placeholder="留空=不去除" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('columnFilter')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="行数据条件过滤" name="rowFilter">
                <el-form-item label="目标列">
                  <el-select v-model="rowFilterCol" placeholder="选择列" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item label="操作符">
                  <el-select v-model="rowFilterOp">
                    <el-option v-for="op in rowFilterOpOptions" :key="op.value" :label="op.label" :value="op.value" />
                  </el-select>
                </el-form-item>
                <el-form-item label="条件值">
                  <el-input v-model="rowFilterVal" :disabled="!rowFilterNeedsValue(rowFilterOp)" placeholder="输入过滤值" />
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('rowFilter')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="填充缺失值" name="fillna">
                <el-form-item label="目标列">
                  <el-select v-model="fillnaCol" placeholder="选择列" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item label="填充值">
                  <el-input v-model="fillnaVal" placeholder="输入填充值" />
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('fillna')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="去重" name="dedup">
                <el-form-item label="去重列">
                  <el-select v-model="dedupCols" multiple placeholder="空 = 全列去重" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('dedup')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="去除前后空格" name="trim">
                <el-form-item label="目标列">
                  <el-select v-model="trimCols" multiple placeholder="选择字符串列" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('trim')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="查找替换" name="findReplace">
                <el-form-item label="目标列">
                  <el-select v-model="frCols" multiple placeholder="选择列" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item label="查找">
                  <el-input v-model="findText" placeholder="查找文本" />
                </el-form-item>
                <el-form-item label="替换为">
                  <el-input v-model="replaceText" placeholder="替换文本" />
                </el-form-item>
                <el-form-item label="正则表达式">
                  <el-switch v-model="useRegex" />
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('findReplace')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>

              <el-collapse-item title="类型转换" name="typeCast">
                <el-form-item label="目标列">
                  <el-select v-model="typeCol" placeholder="选择列" clearable>
                    <el-option v-for="c in dataStore.columnNames" :key="c" :label="c" :value="c" />
                  </el-select>
                </el-form-item>
                <el-form-item label="目标类型">
                  <el-select v-model="typeTarget">
                    <el-option label="整数 (int)" value="int" />
                    <el-option label="浮点 (float)" value="float" />
                    <el-option label="字符串 (str)" value="str" />
                    <el-option label="日期时间 (datetime)" value="datetime" />
                    <el-option label="日期 (date)" value="date" />
                  </el-select>
                </el-form-item>
                <el-form-item class="section-actions">
                  <el-button class="action-btn" type="primary" :loading="cleanLoading"
                    @click="applySectionClean('typeCast')">应用当前项</el-button>
                  <el-button class="action-btn" :loading="cleanLoading" @click="undoCleanStep">撤销一步</el-button>
                </el-form-item>
              </el-collapse-item>
            </el-collapse>

            <el-form-item class="clean-actions">
              <el-button class="action-btn clean-btn-main" type="primary" :loading="cleanLoading" @click="applyClean">
                应用清洗
              </el-button>
              <el-button class="action-btn" type="warning" :loading="cleanLoading" @click="rollbackClean">
                回退数据
              </el-button>
              <el-button class="action-btn" @click="resetClean">
                重置
              </el-button>
            </el-form-item>
          </el-form>

          <!-- ↓ 导出数据 -->
          <el-divider class="section-divider" />
          <div class="section-title-row">
            <span class="section-label">↓ 导出数据</span>
          </div>
          <el-form label-width="0" size="small" :disabled="!dataStore.hasData">
            <el-form-item class="export-actions">
              <el-button class="action-btn" type="success" :loading="saveLoading" :disabled="!dataStore.hasData"
                @click="exportFile('csv')">
                导出 CSV
              </el-button>
              <el-button class="action-btn" type="warning" :loading="saveLoading" :disabled="!dataStore.hasData"
                @click="exportFile('xlsx')">
                导出 Excel
              </el-button>
            </el-form-item>
            <el-text type="info" size="small">导出当前全量数据（非预览行）</el-text>
          </el-form>
        </el-card>
        </div>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </el-col>

      <!-- 右侧：数据预览表格 -->
      <el-col :span="contentSpan" class="content-col">
        <el-card class="panel-card preview-card" :header="`数据预览（${previewRows.length} 行${cleanedPayload ? ' — 已清洗' : ''}）`" shadow="never">
          <div v-if="previewRows.length === 0" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="80" />
          </div>
          <div v-else class="table-wrapper">
            <div class="table-info">
              <el-text type="info" size="small">
                {{ previewRows.length }} 行（总计 {{ dataStore.payload?.total_rows }} 行）
              </el-text>
            </div>
            <el-table :data="previewRows" border stripe size="small" style="width: 100%"
              :default-sort="{ prop: '', order: null }" height="100%">
              <el-table-column v-for="col in tableColumns" :key="col.name" :prop="col.name" :label="col.name"
                min-width="120" show-overflow-tooltip>
                <template #header>
                  <div class="col-header">
                    <span>{{ col.name }}</span>
                    <el-tag size="small" type="info">{{ col.dtype }}</el-tag>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<style scoped>
.load-clean-view {
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

.panel-collapse-btn {
  font-size: 16px;
  padding: 0;
  line-height: 1;
  height: auto;
}

.toggle-all-btn {
  font-size: 12px;
  padding: 0;
  height: auto;
}

.config-unified-card {
  flex-shrink: 0;
}

.config-unified-card :deep(.el-card__body) {
  padding: 12px 16px;
}

.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.section-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.section-divider {
  margin: 16px 0 14px;
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
  display: flex;
  flex-direction: column;
  padding-right: 2px;
}

.preview-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.preview-card :deep(.el-card__body) {
  flex: 1;
  min-height: 0;
  padding: 8px 12px;
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

.hint {
  color: var(--el-text-color-secondary);
  margin-top: 4px;
  font-size: 11px;
}

.col-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.table-wrapper {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.table-info {
  padding: 8px 0;
  border-bottom: 1px solid var(--el-border-color-light);
}

.action-btn {
  height: 28px;
  padding: 0 12px;
  font-size: 13px;
  font-weight: 500;
  margin-left: 0 !important;
}

.load-btn {
  width: 100%;
}

.clean-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.clean-actions .action-btn {
  flex: 1 1 110px;
}

.section-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  gap: 10px;
}

.section-actions .action-btn {
  flex: 1 1 120px;
}

.clean-collapse {
  margin-bottom: 16px;
  border-top: 1px solid var(--el-border-color-light);
  border-bottom: 1px solid var(--el-border-color-light);
}

.clean-collapse :deep(.el-collapse-item__header) {
  font-weight: 600;
  background: transparent;
}

.clean-collapse :deep(.el-collapse-item__wrap) {
  background: transparent;
}

.clean-btn-main {
  flex: 1.5 1 170px;
}

.export-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.export-actions .action-btn {
  flex: 1 1 180px;
}

@media (max-width: 1280px) {

  .clean-actions .action-btn,
  .export-actions .action-btn {
    flex: 1 1 100%;
  }

  .clean-btn-main {
    flex: 1 1 100%;
  }
}
</style>
