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

import { shallowRef, ref, computed, watch, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { save as saveDialog } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import type { ChartPayload, DatasetMeta } from '../utils/chartAdapter'
import { getBusinessColumnLabel, getBusinessOptionLabel } from '../utils/businessColumnLabels'
import { useResize } from '../composables/useResize'
import { usePathUpload } from '../composables/usePathUpload'

const dataStore = useDataStore()
const upload = usePathUpload()

type ViewMode = 'all' | 'load' | 'clean'
const props = withDefaults(defineProps<{ mode?: ViewMode }>(), {
  mode: 'all',
})

// ─── 状态 ─────────────────────────────────────────────────────────────────────

const filePaths = ref<string[]>([])
const skipHead = ref(0)
const skipTail = ref(0)
const headerRow = ref(-1)
const lockHeaderRow = ref(false)
const loading = ref(false)
const loadNotices = ref<string[]>([])
const selectedDatasetId = ref('')
const childDatasetName = ref('')
const sourceType = ref<'file' | 'sql' | 'gsheets' | 'api'>('file')

const sqlConnectionString = ref('')
const sqlQuery = ref('SELECT * FROM your_table LIMIT 1000')
const sqlDatasetName = ref('')

const gsheetSpreadsheet = ref('')
const gsheetGid = ref('')
const gsheetProxyUrl = ref('')
const gsheetDatasetName = ref('')

const GSHEET_PROXY_STORAGE_KEY = 'bi.datasource.gsheet.proxyUrl'

const apiUrl = ref('')
const apiAuthType = ref<'none' | 'bearer' | 'api_key'>('none')
const apiAuthValue = ref('')
const apiDatasetName = ref('')

watch(headerRow, (v) => {
  if (v < 0) {
    lockHeaderRow.value = false
  }
})

watch(gsheetProxyUrl, (val) => {
  try {
    const cleaned = val.trim()
    if (cleaned) {
      window.localStorage.setItem(GSHEET_PROXY_STORAGE_KEY, cleaned)
    } else {
      window.localStorage.removeItem(GSHEET_PROXY_STORAGE_KEY)
    }
  } catch {
    // Ignore persistence errors so datasource loading is not blocked.
  }
})

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
const typeCols = ref<string[]>([])
const typeTarget = ref<'int' | 'float' | 'str' | 'datetime' | 'date'>('str')

// 清洗后预览数据
const cleanedPayload = shallowRef<ChartPayload | null>(null)
const cleanLoading = ref(false)
const configCollapsed = ref(false)

// ─── 排序状态 ─────────────────────────────────────────────────────────────────
const sortCol = ref('')
const sortAsc = ref(true)
const sortedRows = ref<Record<string, any>[]>([])

const { configWidth, startResize } = useResize(320, 640)
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

const showLoadSection = computed(() => props.mode === 'all' || props.mode === 'load')
const showCleanSection = computed(() => props.mode === 'all' || props.mode === 'clean')
const showExportSection = computed(() => props.mode === 'all' || props.mode === 'clean')
const panelTitle = computed(() => {
  if (props.mode === 'load') return '数据加载'
  if (props.mode === 'clean') return '数据清洗'
  return '数据处理'
})

// ─── 计算属性 ─────────────────────────────────────────────────────────────────

// 用于 el-table 的列定义（来自已加载的 payload）
const tableColumns = computed(
  () => (cleanedPayload.value ?? dataStore.payload)?.columns ?? []
)

// 预览的行（优先显示排序后，其次清洗后，最后原始）
const previewRows = computed(
  () => {
    if (sortedRows.value.length > 0) {
      return sortedRows.value
    }
    return (cleanedPayload.value ?? dataStore.payload)?.rows ?? []
  }
)

const previewDatasetName = computed(() => {
  const activeId = dataStore.activeDatasetId
  const fallbackId = selectedDatasetId.value
  const targetId = activeId || fallbackId
  if (targetId) {
    const hit = dataStore.datasets.find((d) => d.id === targetId)
    if (hit?.name) return hit.name
  }
  if (filePaths.value.length > 0) {
    const first = filePaths.value[0]
    return first.split('/').pop() || first
  }
  return '数据预览'
})

const previewHeaderTitle = computed(() => {
  const totalRows = (cleanedPayload.value ?? dataStore.payload)?.total_rows ?? previewRows.value.length
  return `${previewDatasetName.value} · 预览 ${previewRows.value.length}/${totalRows} 行${cleanedPayload.value ? ' — 已清洗' : ''}`
})

function normalizeInputPaths(paths: string[]): string[] {
  return Array.from(new Set(paths.map((p) => p.trim()).filter(Boolean)))
}

async function onDropZoneDrop(e: DragEvent) {
  const droppedPaths = upload.onDrop(e)
  if (droppedPaths.length === 0) {
    ElMessage.warning('未识别到可加载的文件/文件夹路径，请点击拖拽框选择')
    return
  }
  await loadFiles(droppedPaths)
}

async function selectFile() {
  try {
    const selectedPaths = await upload.pickByClick()
    if (selectedPaths.length === 0) return
    await loadFiles(selectedPaths)
  } catch (e: any) {
    ElMessage.error(`文件选择失败: ${String(e)}`)
  }
}

// ─── 加载文件（单文件 / 多文件） ─────────────────────────────────────────────

async function loadFiles(inputPaths?: string[]) {
  const candidates = normalizeInputPaths(inputPaths ?? filePaths.value)
  if (candidates.length === 0) {
    ElMessage.warning('请先选择文件或文件夹')
    return
  }

  filePaths.value = candidates
  loading.value = true
  cleanedPayload.value = null
  loadNotices.value = []
  const perfLabel = '📊 数据加载性能'
  try {
    console.time(perfLabel)
    console.log('📥 开始 Tauri IPC 调用...')

    const t1 = performance.now()
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('load_paths_as_datasets', {
      paths: candidates,
      skipHead: skipHead.value,
      skipTail: skipTail.value,
      headerRow: headerRow.value,
      headerLocked: lockHeaderRow.value,
    })
    const t2 = performance.now()
    console.log(`⏱️  Tauri IPC: ${(t2 - t1).toFixed(2)}ms`)

    if (result.ok && result.data) {
      console.log(`📦 数据包大小: ${result.data.total_rows} 行 × ${result.data.columns.length} 列`)
      console.log(`📝 预览行数: ${result.data.rows.length} 行`)

      const t3 = performance.now()
      dataStore.setPayload(result.data)
      loadNotices.value = result.data.notices ?? []
      await refreshDatasetList()
      if (dataStore.datasets.length > 0) {
        const activeId = dataStore.datasets[0].id
        dataStore.setActiveDatasetId(activeId)
        selectedDatasetId.value = activeId
      }
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
      typeCols.value = []
      console.timeEnd(perfLabel)
      const filesLabel = candidates.length > 1 ? `${candidates.length} 个路径` : '1 个路径'
      ElMessage.success(`✅ 数据加载成功（${filesLabel}），共 ${result.data.total_rows} 行（预览 ${result.data.rows.length} 行）`)
      if (loadNotices.value.length > 0) {
        ElMessage.warning(`检测到 ${loadNotices.value.length} 项需手工处理，详情见信息栏`)
      }
    } else {
      ElMessage.error(result.error ?? '加载失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function applyLoadedResult(result: { ok: boolean; data?: ChartPayload; error?: string }, fallbackName?: string) {
  if (result.ok && result.data) {
    dataStore.setPayload(result.data)
    loadNotices.value = result.data.notices ?? []
    await refreshDatasetList()
    if (dataStore.datasets.length > 0) {
      const matched = dataStore.datasets.find((d) => !fallbackName || d.name.includes(fallbackName))
      const activeId = matched?.id ?? dataStore.datasets[0].id
      dataStore.setActiveDatasetId(activeId)
      selectedDatasetId.value = activeId
    }
    cleanedPayload.value = null
    ElMessage.success(`✅ 数据加载成功，共 ${result.data.total_rows} 行（预览 ${result.data.rows.length} 行）`)
    if (loadNotices.value.length > 0) {
      ElMessage.warning(`检测到 ${loadNotices.value.length} 项需手工处理，详情见信息栏`)
    }
  } else {
    ElMessage.error(result.error ?? '加载失败')
  }
}

async function connectSqlDataset() {
  if (!sqlConnectionString.value.trim()) {
    ElMessage.warning('请输入 SQL 连接字符串')
    return
  }
  if (!sqlQuery.value.trim()) {
    ElMessage.warning('请输入 SQL 查询语句')
    return
  }

  loading.value = true
  loadNotices.value = []
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('load_sql_dataset', {
      connectionString: sqlConnectionString.value,
      query: sqlQuery.value,
      datasetName: sqlDatasetName.value,
    })
    await applyLoadedResult(result, sqlDatasetName.value.trim())
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function connectGoogleSheetDataset() {
  if (!gsheetSpreadsheet.value.trim()) {
    ElMessage.warning('请输入 Google Sheets 链接或 ID')
    return
  }

  loading.value = true
  loadNotices.value = []
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('load_google_sheet_dataset', {
      spreadsheet: gsheetSpreadsheet.value,
      gid: gsheetGid.value,
      proxyUrl: gsheetProxyUrl.value,
      datasetName: gsheetDatasetName.value,
    })
    await applyLoadedResult(result, gsheetDatasetName.value.trim())
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function connectHttpApiDataset() {
  if (!apiUrl.value.trim()) {
    ElMessage.warning('请输入 API URL')
    return
  }

  loading.value = true
  loadNotices.value = []
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('load_http_api_dataset', {
      url: apiUrl.value,
      authType: apiAuthType.value,
      authValue: apiAuthValue.value,
      datasetName: apiDatasetName.value,
    })
    await applyLoadedResult(result, apiDatasetName.value.trim())
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function connectBySourceType() {
  if (sourceType.value === 'file') {
    await loadFiles()
    return
  }
  if (sourceType.value === 'sql') {
    await connectSqlDataset()
    return
  }
  if (sourceType.value === 'gsheets') {
    await connectGoogleSheetDataset()
    return
  }
  await connectHttpApiDataset()
}

async function refreshDatasetList() {
  try {
    const result: { ok: boolean; data?: DatasetMeta[]; error?: string } = await invoke('list_datasets')
    if (result.ok && result.data) {
      dataStore.setDatasets(result.data)
      if (!selectedDatasetId.value && result.data.length > 0) {
        selectedDatasetId.value = result.data[0].id
      }
    } else {
      ElMessage.warning(result.error ?? '读取数据列表失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  }
}

async function switchDataset() {
  if (!selectedDatasetId.value) {
    ElMessage.warning('请先选择一个数据集')
    return
  }

  loading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('switch_dataset', {
      datasetId: selectedDatasetId.value,
    })
    if (result.ok && result.data) {
      dataStore.setPayload(result.data)
      dataStore.setActiveDatasetId(selectedDatasetId.value)
      cleanedPayload.value = null
      ElMessage.success('已切换当前分析数据')
    } else {
      ElMessage.error(result.error ?? '切换失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function saveCurrentAsDataset() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  try {
    const result: { ok: boolean; data?: DatasetMeta; error?: string } = await invoke('save_current_dataset', {
      name: childDatasetName.value,
      source: 'load_clean',
    })
    if (result.ok && result.data) {
      ElMessage.success('已保存到数据列表')
      childDatasetName.value = ''
      await refreshDatasetList()
      selectedDatasetId.value = result.data.id
      dataStore.setActiveDatasetId(result.data.id)
    } else {
      ElMessage.error(result.error ?? '保存失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  }
}

async function deleteSelectedDataset() {
  if (!selectedDatasetId.value) {
    ElMessage.warning('请先选择要删除的数据集')
    return
  }
  try {
    const result: { ok: boolean; data?: DatasetMeta[]; error?: string } = await invoke('delete_datasets', {
      datasetIds: [selectedDatasetId.value],
    })
    if (result.ok && result.data) {
      dataStore.setDatasets(result.data)
      const remainingIds = result.data.map((d) => d.id)
      if (!remainingIds.includes(selectedDatasetId.value)) {
        selectedDatasetId.value = result.data[0]?.id ?? ''
        // 若活跃数据集被删，同步 store
        if (!remainingIds.includes(dataStore.activeDatasetId)) {
          if (result.data.length > 0) {
            dataStore.setActiveDatasetId(result.data[0].id)
          } else {
            dataStore.clear()
          }
        }
      }
      ElMessage.success('已删除数据集')
    } else {
      ElMessage.error(result.error ?? '删除失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  }
}

onMounted(() => {
  try {
    const cachedProxy = window.localStorage.getItem(GSHEET_PROXY_STORAGE_KEY)
    if (cachedProxy && cachedProxy.trim()) {
      gsheetProxyUrl.value = cachedProxy.trim()
    }
  } catch {
    // Ignore restore errors so page can still initialize.
  }
  refreshDatasetList()
})

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
      typeCols: typeCols.value,
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
    typeCols: section === 'typeCast' ? typeCols.value : [],
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
  if (section === 'typeCast' && typeCols.value.length === 0) {
    ElMessage.warning('请至少选择一列进行类型转换')
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

// ─── 重置清洗 ────────────────────────────────────────────────────────────────

function resetCleanOptions() {
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
  typeCols.value = []
  typeTarget.value = 'str'
}

async function restoreInitialData() {
  // Always reset UI options first so the panel state is deterministic.
  resetCleanOptions()

  if (!dataStore.hasData) {
    ElMessage.success('清洗参数已重置')
    return
  }

  cleanLoading.value = true
  try {
    const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke('rollback_clean')
    if (result.ok && result.data) {
      dataStore.setPayload(result.data)
      cleanedPayload.value = null
      ElMessage.success('已恢复到加载时初始数据，并重置清洗参数')
    } else {
      ElMessage.error(result.error ?? '恢复失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    cleanLoading.value = false
  }
}

// ─── 排序功能 ──────────────────────────────────────────────────────────────────

function handleTableSort(evt: any) {
  const { prop, order } = evt
  
  if (!prop || !order) {
    // 清除排序
    sortCol.value = ''
    sortedRows.value = []
    return
  }

  sortCol.value = prop
  sortAsc.value = order === 'ascending'
  
  // 对预览行进行排序
  const rows = previewRows.value
  const sorted = [...rows].sort((a, b) => {
    const aVal = a[prop]
    const bVal = b[prop]
    
    // 处理 null/undefined
    if (aVal == null && bVal == null) return 0
    if (aVal == null) return sortAsc.value ? 1 : -1
    if (bVal == null) return sortAsc.value ? -1 : 1
    
    // 数值比较
    if (typeof aVal === 'number' && typeof bVal === 'number') {
      return sortAsc.value ? aVal - bVal : bVal - aVal
    }
    
    // 字符串比较
    const aStr = String(aVal)
    const bStr = String(bVal)
    const cmp = aStr.localeCompare(bStr)
    return sortAsc.value ? cmp : -cmp
  })
  
  sortedRows.value = sorted
}

async function saveSortedDataset() {
  if (!dataStore.hasData) {
    ElMessage.warning('请先加载数据')
    return
  }
  
  if (!sortCol.value) {
    ElMessage.warning('请先选择排序列')
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
      ElMessage.success(`已保存排序结果为新数据集: ${result.data.name}`)
      // 刷新数据集列表
      await refreshDatasetList()
    } else {
      ElMessage.error(result.error ?? '保存排序结果失败')
    }
  } catch (e: any) {
    ElMessage.error(String(e))
  } finally {
    saveLoading.value = false
  }
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
    <div class="layout-row">
      <!-- 左侧：加载 + 清洗参数面板 -->
      <div class="config-col"
        :style="configCollapsed ? { width: '28px', minWidth: '28px' } : { width: configWidth + 'px', minWidth: configWidth + 'px' }">
        <div v-if="!configCollapsed" class="config-scroll">
          <el-card class="panel-card config-unified-card" shadow="never">
            <template #header>
              <div class="panel-header">
                <span>{{ panelTitle }}</span>
                <el-button text class="panel-collapse-btn" title="收起" @click="configCollapsed = true">‹</el-button>
              </div>
            </template>

            <!-- ① 数据加载 -->
            <template v-if="showLoadSection">
            <div class="section-title-row">
              <span class="section-label">① 数据加载</span>
            </div>
            <el-form class="compact-form" label-width="72px" label-position="left" size="small">
              <el-form-item label="连接方式">
                <el-radio-group v-model="sourceType">
                  <el-radio-button label="file">上传 Excel/CSV</el-radio-button>
                  <el-radio-button label="sql">连接 SQL</el-radio-button>
                  <el-radio-button label="gsheets">Google Sheets</el-radio-button>
                  <el-radio-button label="api">自定义 API</el-radio-button>
                </el-radio-group>
              </el-form-item>

              <template v-if="sourceType === 'file'">
              <el-form-item label="拖拽导入">
                <div class="drop-zone" :class="{ 'is-drag-over': upload.dragOver, 'is-loading': loading }"
                  @click="selectFile" @dragenter.prevent="upload.onDragEnter" @dragover.prevent="upload.onDragEnter"
                  @dragleave.prevent="upload.onDragLeave" @drop.prevent="onDropZoneDrop">
                  <div class="drop-zone-title">拖拽 CSV/Excel 文件或文件夹到此处</div>
                  <div class="drop-zone-sub">或点击这里选择文件/文件夹，按文件逐个入库并预览首文件</div>
                  <div v-if="filePaths.length" class="drop-zone-path">
                    已选 {{ filePaths.length }} 个路径：{{ filePaths[0] }}<span v-if="filePaths.length > 1"> 等</span>
                  </div>
                </div>
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
              <el-form-item label="锁定表头行">
                <el-checkbox v-model="lockHeaderRow" :disabled="headerRow < 0">
                  锁定后，跳过开头行从表头下一行开始计算
                </el-checkbox>
              </el-form-item>
              <el-form-item class="single-action">
                <el-button
                  class="action-btn"
                  type="primary"
                  :loading="loading"
                  :disabled="filePaths.length === 0"
                  @click="connectBySourceType"
                >{{ dataStore.hasData ? '重新加载' : '加载数据' }}</el-button>
              </el-form-item>
              </template>

              <template v-else-if="sourceType === 'sql'">
                <el-form-item label="连接串">
                  <el-input v-model="sqlConnectionString" placeholder="支持 sqlite:///path.db、mysql://user:pass@host:3306/db、postgresql://user:pass@host:5432/db" />
                </el-form-item>
                <el-form-item label="SQL 查询">
                  <el-input v-model="sqlQuery" type="textarea" :rows="4" placeholder="SELECT * FROM your_table LIMIT 1000" />
                </el-form-item>
                <el-form-item label="数据集名">
                  <el-input v-model="sqlDatasetName" placeholder="可选，留空自动命名" />
                </el-form-item>
                <el-form-item class="single-action">
                  <el-button class="action-btn" type="primary" :loading="loading" @click="connectBySourceType">连接并加载</el-button>
                </el-form-item>
              </template>

              <template v-else-if="sourceType === 'gsheets'">
                <el-form-item label="表格链接">
                  <el-input v-model="gsheetSpreadsheet" placeholder="Google Sheets URL 或 spreadsheet ID" />
                </el-form-item>
                <el-form-item label="工作表GID">
                  <el-input v-model="gsheetGid" placeholder="可选，指定工作表 gid" />
                </el-form-item>
                <el-form-item label="代理地址">
                  <el-input v-model="gsheetProxyUrl" placeholder="可选，如 http://127.0.0.1:7890" />
                </el-form-item>
                <el-form-item label="数据集名">
                  <el-input v-model="gsheetDatasetName" placeholder="可选，留空自动命名" />
                </el-form-item>
                <el-form-item class="single-action">
                  <el-button class="action-btn" type="primary" :loading="loading" @click="connectBySourceType">连接并加载</el-button>
                </el-form-item>
              </template>

              <template v-else>
                <el-form-item label="API URL">
                  <el-input v-model="apiUrl" placeholder="https://example.com/data" />
                </el-form-item>
                <el-form-item label="认证方式">
                  <el-select v-model="apiAuthType">
                    <el-option label="无" value="none" />
                    <el-option label="Bearer Token" value="bearer" />
                    <el-option label="API Key (X-API-Key)" value="api_key" />
                  </el-select>
                </el-form-item>
                <el-form-item label="认证值">
                  <el-input v-model="apiAuthValue" :placeholder="apiAuthType === 'none' ? '无需填写' : '输入认证值'" />
                </el-form-item>
                <el-form-item label="数据集名">
                  <el-input v-model="apiDatasetName" placeholder="可选，留空自动命名" />
                </el-form-item>
                <el-form-item class="single-action">
                  <el-button class="action-btn" type="primary" :loading="loading" @click="connectBySourceType">连接并加载</el-button>
                </el-form-item>
              </template>

              <el-divider class="dataset-block-divider" />

              <el-form-item label="数据列表">
                <el-select v-model="selectedDatasetId" placeholder="选择数据集" clearable>
                  <el-option v-for="d in dataStore.datasets" :key="d.id"
                    :label="`${d.name} (${d.total_rows}x${d.total_cols})`" :value="d.id" />
                </el-select>
              </el-form-item>
              <el-form-item>
                <div class="dataset-row-actions">
                  <el-button class="action-btn" @click="refreshDatasetList">刷新</el-button>
                  <el-button class="action-btn" type="primary" @click="switchDataset">切换</el-button>
                  <el-button class="action-btn" type="danger" :disabled="!selectedDatasetId"
                    @click="deleteSelectedDataset">删除</el-button>
                </div>
              </el-form-item>
              <el-form-item label="子数据名">
                <el-input v-model="childDatasetName" placeholder="可选，留空自动命名" />
              </el-form-item>
              <el-form-item class="single-action">
                <el-button class="action-btn" type="success" @click="saveCurrentAsDataset">保存当前到列表</el-button>
              </el-form-item>
            </el-form>
            </template>

            <!-- ② 数据清洗 -->
            <template v-if="showCleanSection">
            <el-divider class="section-divider" />
            <div class="section-title-row">
              <span class="section-label">② 数据清洗</span>
              <el-button text class="toggle-all-btn"
                @click="activeCleanSections = activeCleanSections.length ? [] : ['columnFilter', 'rowFilter', 'fillna', 'dedup', 'trim', 'findReplace', 'typeCast']">
                {{ activeCleanSections.length ? '全部折叠' : '全部展开' }}
              </el-button>
            </div>
            <el-form class="compact-form" label-width="72px" label-position="left" size="small"
              :disabled="!dataStore.hasData">
              <el-collapse v-model="activeCleanSections" class="clean-collapse">
                <el-collapse-item title="去除列" name="columnFilter">
                  <el-form-item label="去除列">
                    <el-select v-model="filterCols" multiple placeholder="留空=不去除" clearable>
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="操作符">
                    <el-select v-model="rowFilterOp">
                      <el-option v-for="op in rowFilterOpOptions" :key="op.value" :label="op.label" :value="op.value" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="条件值">
                    <el-input v-model="rowFilterVal" :disabled="!rowFilterNeedsValue(rowFilterOp)"
                      placeholder="输入过滤值" />
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
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                    <el-select v-model="typeCols" multiple placeholder="选择一列或多列" clearable>
                      <el-option v-for="c in dataStore.columnNames" :key="c" :label="getBusinessOptionLabel(c)" :value="c" />
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
                <el-button class="action-btn" type="warning" :loading="cleanLoading" @click="restoreInitialData">
                  恢复初始
                </el-button>
              </el-form-item>
            </el-form>
            </template>

            <!-- ↓ 导出数据 -->
            <template v-if="showExportSection">
            <el-divider class="section-divider" />
            <div class="section-title-row">
              <span class="section-label">↓ 导出数据</span>
            </div>
            <el-form label-width="0" size="small" :disabled="!dataStore.hasData">
              <el-form-item class="export-actions">
                <div class="export-btn-row">
                  <el-button class="action-btn export-btn" size="small" type="success" :loading="saveLoading"
                    :disabled="!dataStore.hasData" @click="exportFile('csv')">
                    ⤓ CSV
                  </el-button>
                  <el-button class="action-btn export-btn" size="small" type="warning" :loading="saveLoading"
                    :disabled="!dataStore.hasData" @click="exportFile('xlsx')">
                    ⤓ Excel
                  </el-button>
                </div>
              </el-form-item>
              <el-text type="info" size="small">导出当前全量数据（非预览行）</el-text>
            </el-form>
            </template>
          </el-card>
        </div>

        <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
      </div>
      <div v-if="!configCollapsed" class="resize-handle" @mousedown.prevent="startResize" />

      <!-- 右侧：数据预览表格 -->
      <div class="content-col">
        <el-card class="panel-card preview-card" :header="previewHeaderTitle" shadow="never">
          <div v-if="previewRows.length === 0" class="display-empty">
            <el-empty description="暂无数据，请先加载数据" :image-size="80" />
          </div>
          <div v-else class="table-wrapper">
            <div class="table-info">
              <div class="table-info-main">
                <el-text type="info" size="small">
                  {{ previewRows.length }} 行（总计 {{ dataStore.payload?.total_rows }} 行）
                </el-text>
                <div v-if="sortCol" class="sort-status" style="margin-left: 12px;">
                  <el-text type="success" size="small">
                    📊 已按 <strong>{{ sortCol }}</strong> {{ sortAsc ? '升序' : '降序' }} 排序
                  </el-text>
                  <el-button link type="primary" size="small" @click="saveSortedDataset" :loading="saveLoading"
                    style="margin-left: 8px;">
                    💾 保存排序结果
                  </el-button>
                  <el-button link type="info" size="small" @click="() => { sortCol = ''; sortedRows = []; }"
                    style="margin-left: 4px;">
                    ✕ 清除排序
                  </el-button>
                </div>
              </div>
              <div v-if="loadNotices.length > 0" class="table-info-notices">
                <el-alert type="warning" show-icon :closable="false" title="以下列未能自动转换为浮点数，请手工处理">
                  <template #default>
                    <div class="notice-line" v-for="(msg, idx) in loadNotices" :key="`${idx}-${msg}`">
                      {{ msg }}
                    </div>
                  </template>
                </el-alert>
              </div>
            </div>
            <el-table :data="previewRows" border stripe size="small" style="width: 100%"
              :default-sort="{ prop: '', order: null }" height="100%" @sort-change="handleTableSort">
              <el-table-column v-for="col in tableColumns" :key="col.name" :prop="col.name" :label="getBusinessColumnLabel(col.name)"
                sortable="custom" min-width="120" show-overflow-tooltip>
                <template #header>
                  <div class="col-header">
                    <span>{{ getBusinessColumnLabel(col.name) }}</span>
                    <el-tag size="small" type="info">{{ col.dtype }}</el-tag>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </el-card>
      </div>
    </div>
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

.table-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.compact-form :deep(.el-form-item) {
  margin-bottom: 10px;
}

.dataset-row-actions {
  width: 100%;
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.notice-line {
  line-height: 1.4;
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

.dataset-block-divider {
  margin: 6px 0 10px;
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

.drop-zone {
  width: 100%;
  border: 1px dashed var(--el-border-color);
  background: rgba(64, 158, 255, 0.06);
  border-radius: 8px;
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.drop-zone:hover {
  border-color: var(--el-color-primary-light-5);
  background: var(--el-color-primary-light-9);
}

.drop-zone.is-drag-over {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-8);
}

.drop-zone.is-loading {
  opacity: 0.75;
  pointer-events: none;
}

.drop-zone-title {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.4;
}

.drop-zone-sub {
  margin-top: 2px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.drop-zone-path {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-text-color-regular);
  word-break: break-all;
}

.export-actions :deep(.el-form-item__content) {
  width: 100%;
}

.export-btn-row {
  width: 100%;
  display: flex;
  flex-wrap: nowrap;
  justify-content: flex-end;
  gap: 8px;
}

.export-btn {
  width: 72px;
  flex: none;
  height: 30px;
  font-size: 12px;
  padding: 0 8px;
}

.single-action :deep(.el-form-item__content) {
  justify-content: flex-end;
}

.clean-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}

.clean-actions .action-btn {
  width: 88px;
  flex: none;
}

.section-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.section-actions .action-btn {
  width: 88px;
  flex: none;
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
  width: 88px;
  flex: none;
}

.export-actions :deep(.el-form-item__content) {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}

.export-actions .action-btn {
  width: 88px;
  flex: none;
}
</style>
