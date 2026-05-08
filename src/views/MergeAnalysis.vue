<script setup lang="ts">
// src/views/MergeAnalysis.vue
// 数据表合并视图（JOIN / CONCAT）
//
// 功能：
//   1. JOIN 模式  — 将当前活跃表与注册数据集横向连接（inner / left / right / outer）
//   2. CONCAT 模式 — 将多个数据集纵向堆叠（严格模式 / 宽松模式）

import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { useDataStore } from '../stores/dataStore'
import type { ChartPayload } from '../utils/chartAdapter'
import { useResize } from '../composables/useResize'
import { useDatasetActions } from '../composables/useDatasetActions'
import { usePathUpload } from '../composables/usePathUpload'

const dataStore = useDataStore()
const { configWidth, startResize } = useResize(320, 600)
const { loadDatasets } = useDatasetActions()
const upload = usePathUpload()

// ─── 通用状态 ─────────────────────────────────────────────────────────────────

const loading = ref(false)
const configCollapsed = ref(false)
const activeTab = ref<'join' | 'concat'>('join')
const resultPayload = ref<ChartPayload | null>(null)
const saveDatasetName = ref('')

// ─── JOIN 参数 ────────────────────────────────────────────────────────────────

const rightDatasetId = ref('')
const joinHow = ref<'inner' | 'left' | 'right' | 'outer'>('inner')

// 连接键对：[{ leftCol, rightCol }]
interface KeyPair { leftCol: string; rightCol: string }
const keyPairs = ref<KeyPair[]>([{ leftCol: '', rightCol: '' }])

function addKeyPair() {
    keyPairs.value.push({ leftCol: '', rightCol: '' })
}
function removeKeyPair(idx: number) {
    if (keyPairs.value.length > 1) keyPairs.value.splice(idx, 1)
}

// 右表的列名（从注册表查找）
const rightColumns = ref<string[]>([])

function onRightDatasetChange(id: string) {
    rightDatasetId.value = id
    // 通过 list_datasets 的列信息不完整，这里先用 switch_dataset 拿 schema
    // 用另一个 invoke 获取右表列名
    fetchRightColumns(id)
}

async function fetchRightColumns(_datasetId: string) {
    // 右表列名无法从 list_datasets 元数据获取（不含 schema）
    // 用户手动输入右表列名，或通过 join 时后端报错引导
    rightColumns.value = []
}

// ─── CONCAT 参数 ──────────────────────────────────────────────────────────────

const concatDatasetIds = ref<string[]>([])
const includeCurrent = ref(true)
const diagonalMode = ref(false)
const concatSourceMode = ref<'datasets' | 'paths'>('datasets')
const concatInputPaths = ref<string[]>([])

function dedupePaths(paths: string[]): string[] {
    return Array.from(new Set(paths.map((p) => p.trim()).filter(Boolean)))
}

function addConcatPaths(paths: string[]) {
    concatInputPaths.value = dedupePaths([...concatInputPaths.value, ...paths])
}

function removeConcatPath(path: string) {
    concatInputPaths.value = concatInputPaths.value.filter((p) => p !== path)
}

function clearConcatPaths() {
    concatInputPaths.value = []
}

async function onConcatUploadAreaClick() {
    const picked = await upload.pickByClick()
    if (picked.length > 0) addConcatPaths(picked)
}

function onConcatDrop(e: DragEvent) {
    const finalPaths = upload.onDrop(e)
    if (finalPaths.length === 0) {
        ElMessage.warning('未识别到可用路径，请改用“选择文件”或“选择文件夹”')
        return
    }
    addConcatPaths(finalPaths)
    ElMessage.success(`已添加 ${finalPaths.length} 个路径`)
}

// ─── 操作函数 ─────────────────────────────────────────────────────────────────

async function runJoin() {
    if (!dataStore.hasData) {
        ElMessage.warning('请先在"数据加载"页面加载数据作为左表')
        return
    }
    if (!rightDatasetId.value) {
        ElMessage.warning('请选择右表数据集')
        return
    }
    const validPairs = keyPairs.value.filter((p) => p.leftCol && p.rightCol)
    if (validPairs.length === 0) {
        ElMessage.warning('请至少配置一组连接键')
        return
    }

    loading.value = true
    resultPayload.value = null
    try {
        const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
            'join_datasets',
            {
                rightDatasetId: rightDatasetId.value,
                leftOn: validPairs.map((p) => p.leftCol),
                rightOn: validPairs.map((p) => p.rightCol),
                how: joinHow.value,
                saveAsDataset: false,
            }
        )
        if (result.ok && result.data) {
            resultPayload.value = result.data
        } else {
            ElMessage.error(result.error ?? 'JOIN 失败')
        }
    } catch (e) {
        ElMessage.error(String(e))
    } finally {
        loading.value = false
    }
}

async function saveJoinResult() {
    if (!resultPayload.value) {
        ElMessage.warning('请先执行 JOIN')
        return
    }
    const validPairs = keyPairs.value.filter((p) => p.leftCol && p.rightCol)
    loading.value = true
    try {
        const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
            'join_datasets',
            {
                rightDatasetId: rightDatasetId.value,
                leftOn: validPairs.map((p) => p.leftCol),
                rightOn: validPairs.map((p) => p.rightCol),
                how: joinHow.value,
                saveAsDataset: true,
                datasetName: saveDatasetName.value.trim() || undefined,
            }
        )
        if (result.ok) {
            ElMessage.success('JOIN 结果已保存到数据列表')
            saveDatasetName.value = ''
            await loadDatasets()
        } else {
            ElMessage.error(result.error ?? '保存失败')
        }
    } catch (e) {
        ElMessage.error(String(e))
    } finally {
        loading.value = false
    }
}

async function runConcat() {
    if (concatSourceMode.value === 'datasets') {
        if (!includeCurrent.value && concatDatasetIds.value.length < 2) {
            ElMessage.warning('请至少选择两个数据集进行拼接，或勾选"包含当前数据"')
            return
        }
        if (includeCurrent.value && !dataStore.hasData) {
            ElMessage.warning('当前没有活跃数据，请先在"数据加载"页面加载数据')
            return
        }
    } else {
        if (concatInputPaths.value.length === 0) {
            ElMessage.warning('请先拖拽文件/文件夹，或点击选择路径')
            return
        }
    }

    loading.value = true
    resultPayload.value = null
    try {
        const result: { ok: boolean; data?: ChartPayload; error?: string } = concatSourceMode.value === 'datasets'
            ? await invoke('concat_datasets', {
                datasetIds: concatDatasetIds.value,
                includeCurrent: includeCurrent.value,
                diagonal: diagonalMode.value,
                saveAsDataset: false,
            })
            : await invoke('concat_paths', {
                paths: concatInputPaths.value,
                diagonal: diagonalMode.value,
                saveAsDataset: false,
            })
        if (result.ok && result.data) {
            resultPayload.value = result.data
        } else {
            ElMessage.error(result.error ?? 'CONCAT 失败')
        }
    } catch (e) {
        ElMessage.error(String(e))
    } finally {
        loading.value = false
    }
}

async function saveConcatResult() {
    if (!resultPayload.value) {
        ElMessage.warning('请先执行 CONCAT')
        return
    }
    loading.value = true
    try {
        const result: { ok: boolean; data?: ChartPayload; error?: string } = concatSourceMode.value === 'datasets'
            ? await invoke('concat_datasets', {
                datasetIds: concatDatasetIds.value,
                includeCurrent: includeCurrent.value,
                diagonal: diagonalMode.value,
                saveAsDataset: true,
                datasetName: saveDatasetName.value.trim() || undefined,
            })
            : await invoke('concat_paths', {
                paths: concatInputPaths.value,
                diagonal: diagonalMode.value,
                saveAsDataset: true,
                datasetName: saveDatasetName.value.trim() || undefined,
            })
        if (result.ok) {
            ElMessage.success('CONCAT 结果已保存到数据列表')
            saveDatasetName.value = ''
            await loadDatasets()
        } else {
            ElMessage.error(result.error ?? '保存失败')
        }
    } catch (e) {
        ElMessage.error(String(e))
    } finally {
        loading.value = false
    }
}

function resetJoinConfig() {
    rightDatasetId.value = ''
    joinHow.value = 'inner'
    keyPairs.value = [{ leftCol: '', rightCol: '' }]
    saveDatasetName.value = ''
    resultPayload.value = null
    ElMessage.success('JOIN 参数已重置')
}

function resetConcatConfig() {
    concatDatasetIds.value = []
    includeCurrent.value = true
    diagonalMode.value = false
    concatInputPaths.value = []
    saveDatasetName.value = ''
    resultPayload.value = null
    ElMessage.success('CONCAT 参数已重置')
}

onMounted(async () => {
    await loadDatasets()
})
</script>

<template>
    <div class="merge-analysis-view">
        <div class="layout-row">

            <!-- ─── 左侧控制面板 ─── -->
            <div class="config-col" :style="configCollapsed
                ? { width: '28px', minWidth: '28px' }
                : { width: configWidth + 'px', minWidth: configWidth + 'px' }">

                <div v-if="!configCollapsed" class="config-scroll">
                    <el-card class="panel-card" shadow="never">
                        <template #header>
                            <div class="panel-header">
                                <span>合并参数</span>
                                <el-button text class="panel-collapse-btn" title="收起"
                                    @click="configCollapsed = true">‹</el-button>
                            </div>
                        </template>

                        <el-tabs v-model="activeTab" class="merge-tabs" @tab-change="resultPayload = null">
                            <!-- ═══ JOIN 标签页 ═══ -->
                            <el-tab-pane label="JOIN 横向连接" name="join">
                                <el-form class="compact-form merge-form" label-width="70px" label-position="left"
                                    size="small">

                                    <el-form-item label="右表">
                                        <el-select v-model="rightDatasetId" placeholder="选择右表数据集" style="width:100%"
                                            @change="onRightDatasetChange">
                                            <el-option v-for="d in dataStore.datasets" :key="d.id"
                                                :label="`${d.name}（${d.total_rows}行×${d.total_cols}列）`" :value="d.id" />
                                        </el-select>
                                    </el-form-item>

                                    <el-form-item label="连接方式">
                                        <el-radio-group v-model="joinHow" class="join-how-group">
                                            <el-radio-button value="inner">INNER</el-radio-button>
                                            <el-radio-button value="left">LEFT</el-radio-button>
                                            <el-radio-button value="right">RIGHT</el-radio-button>
                                            <el-radio-button value="outer">OUTER</el-radio-button>
                                        </el-radio-group>
                                    </el-form-item>

                                    <!-- 连接键配置 -->
                                    <el-form-item label=" ">
                                        <span class="keys-label">连接键（左表列 → 右表列）</span>
                                    </el-form-item>

                                    <div v-for="(pair, idx) in keyPairs" :key="idx" class="key-pair-row">
                                        <el-select v-model="pair.leftCol" placeholder="左表列" style="flex:1">
                                            <el-option v-for="c in dataStore.columnNames" :key="c" :label="c"
                                                :value="c" />
                                        </el-select>
                                        <span class="key-arrow">→</span>
                                        <el-input v-model="pair.rightCol" placeholder="右表列名" style="flex:1" />
                                        <el-button text type="danger" size="small" :disabled="keyPairs.length <= 1"
                                            @click="removeKeyPair(idx)">✕</el-button>
                                    </div>

                                    <el-form-item label=" " style="margin-top:4px">
                                        <el-button text type="primary" size="small" @click="addKeyPair">
                                            + 添加连接键
                                        </el-button>
                                    </el-form-item>

                                    <el-form-item class="action-row action-row-inline">
                                        <el-button type="primary" :loading="loading" style="width:120px"
                                            @click="runJoin">
                                            执行 JOIN
                                        </el-button>
                                        <el-button plain type="warning" :disabled="loading" style="width:88px"
                                            @click="resetJoinConfig">
                                            重置
                                        </el-button>
                                    </el-form-item>

                                    <el-divider content-position="left" class="result-divider">保存结果</el-divider>

                                    <el-form-item label="名称" class="save-name-row">
                                        <el-input v-model="saveDatasetName" placeholder="可选，留空自动命名" />
                                    </el-form-item>
                                    <el-form-item class="action-row save-action-row">
                                        <el-button type="success" :loading="loading" :disabled="!resultPayload"
                                            style="width:160px" @click="saveJoinResult">
                                            保存 JOIN 结果
                                        </el-button>
                                    </el-form-item>
                                </el-form>
                            </el-tab-pane>

                            <!-- ═══ CONCAT 标签页 ═══ -->
                            <el-tab-pane label="CONCAT 纵向堆叠" name="concat">
                                <el-form class="compact-form merge-form" label-width="82px" label-position="left"
                                    size="small">

                                    <el-form-item label="数据来源">
                                        <el-radio-group v-model="concatSourceMode">
                                            <el-radio-button value="datasets">当前表+数据集</el-radio-button>
                                            <el-radio-button value="paths">拖拽/路径</el-radio-button>
                                        </el-radio-group>
                                    </el-form-item>

                                    <template v-if="concatSourceMode === 'datasets'">
                                        <el-form-item label="包含当前">
                                            <el-switch v-model="includeCurrent" />
                                            <el-text size="small" type="info" style="margin-left:8px">
                                                将当前活跃表作为第一块
                                            </el-text>
                                        </el-form-item>

                                        <el-form-item label="附加数据集">
                                            <el-select v-model="concatDatasetIds" multiple placeholder="选择要堆叠的数据集"
                                                style="width:100%">
                                                <el-option v-for="d in dataStore.datasets" :key="d.id"
                                                    :label="`${d.name}（${d.total_rows}行×${d.total_cols}列）`"
                                                    :value="d.id" />
                                            </el-select>
                                        </el-form-item>
                                    </template>

                                    <template v-else>
                                        <div class="drop-zone" :class="{ active: upload.dragOver }"
                                            @click="onConcatUploadAreaClick" @dragover.prevent="upload.onDragEnter"
                                            @dragleave.prevent="upload.onDragLeave" @drop="onConcatDrop">
                                            <div class="drop-main">拖拽文件夹或多个文件到这里</div>
                                            <div class="drop-sub">或点击此区域选择文件/文件夹，适合多统计表批量汇总</div>
                                        </div>

                                        <div v-if="concatInputPaths.length > 0" class="path-list-wrap">
                                            <div class="path-list-head">
                                                <span>已选路径（{{ concatInputPaths.length }}）</span>
                                                <el-button text type="danger" size="small"
                                                    :disabled="concatInputPaths.length === 0"
                                                    @click="clearConcatPaths">清空</el-button>
                                            </div>
                                            <div class="path-list">
                                                <div v-for="p in concatInputPaths" :key="p" class="path-item">
                                                    <span class="path-text" :title="p">{{ p }}</span>
                                                    <el-button text type="danger" size="small"
                                                        @click="removeConcatPath(p)">移除</el-button>
                                                </div>
                                            </div>
                                        </div>
                                    </template>

                                    <el-form-item label="宽松模式">
                                        <el-switch v-model="diagonalMode" />
                                        <el-text size="small" type="info" style="margin-left:8px">
                                            列不一致时填 null
                                        </el-text>
                                    </el-form-item>

                                    <el-form-item class="action-row action-row-inline">
                                        <el-button type="primary" :loading="loading" style="width:120px"
                                            @click="runConcat">
                                            执行 CONCAT
                                        </el-button>
                                        <el-button plain type="warning" :disabled="loading" style="width:88px"
                                            @click="resetConcatConfig">
                                            重置
                                        </el-button>
                                    </el-form-item>

                                    <el-divider content-position="left" class="result-divider">保存结果</el-divider>

                                    <el-form-item label="名称" class="save-name-row">
                                        <el-input v-model="saveDatasetName" placeholder="可选，留空自动命名" />
                                    </el-form-item>
                                    <el-form-item class="action-row save-action-row">
                                        <el-button type="success" :loading="loading" :disabled="!resultPayload"
                                            style="width:160px" @click="saveConcatResult">
                                            保存 CONCAT 结果
                                        </el-button>
                                    </el-form-item>
                                </el-form>
                            </el-tab-pane>
                        </el-tabs>

                        <el-text v-if="resultPayload" size="small" type="info" style="display:block; margin-top:8px">
                            当前结果：{{ resultPayload.total_rows }} 行 × {{ resultPayload.columns.length }} 列
                        </el-text>
                    </el-card>
                </div>

                <div v-else class="collapsed-handle" title="展开参数" @click="configCollapsed = false">›</div>
            </div>

            <div v-if="!configCollapsed" class="resize-handle" @mousedown.prevent="startResize" />

            <!-- ─── 右侧结果区 ─── -->
            <div class="content-col">
                <el-card class="panel-card result-card"
                    :header="`合并结果（${resultPayload?.total_rows ?? 0} 行 × ${resultPayload?.columns?.length ?? 0} 列）`"
                    shadow="never">
                    <div v-if="!resultPayload" class="display-empty">
                        <el-empty :description="activeTab === 'join'
                            ? '配置左右表与连接键后，点击「执行 JOIN」'
                            : (concatSourceMode === 'datasets'
                                ? '选择数据集后，点击「执行 CONCAT」'
                                : '拖拽文件夹或多个文件后，点击「执行 CONCAT」')" :image-size="80" />
                    </div>
                    <el-table v-else :data="resultPayload.rows" border stripe size="small" style="width:100%"
                        height="100%">
                        <el-table-column v-for="col in resultPayload.columns" :key="col.name" :prop="col.name"
                            :label="col.name" min-width="110" show-overflow-tooltip />
                    </el-table>
                </el-card>
            </div>

        </div>
    </div>
</template>

<style scoped>
.merge-analysis-view {
    height: 100%;
    overflow: hidden;

    /* 统一布局变量：与其他分析页保持同一调优入口 */
    --form-item-gap: 14px;
    --form-item-gap-compact: 10px;
    --group-gap: 18px;
    --divider-gap-top: 20px;
    --divider-gap-bottom: 16px;
    --action-inline-gap: 8px;
    --save-name-gap: 16px;
    --save-action-gap: 18px;
    --tabs-header-gap: 12px;
}

.layout-row {
    height: 100%;
    display: flex;
    overflow: hidden;
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

.result-card {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
}

.result-card :deep(.el-card__body) {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
}

.display-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
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

:deep(.el-card__header) {
    padding: 8px 16px;
}

.compact-form :deep(.el-form-item) {
    margin-bottom: var(--form-item-gap-compact);
}

.merge-form :deep(.el-form-item) {
    margin-bottom: var(--form-item-gap);
}

.merge-form :deep(.el-form-item.action-row-inline) {
    margin-bottom: var(--group-gap);
}

.merge-form :deep(.el-form-item.save-name-row) {
    margin-bottom: var(--save-name-gap);
}

.merge-form :deep(.el-form-item.save-action-row) {
    margin-bottom: var(--save-action-gap);
}

.merge-form :deep(.el-form-item__label) {
    line-height: 32px;
}

.action-row :deep(.el-form-item__content) {
    justify-content: flex-end;
}

.action-row-inline :deep(.el-form-item__content) {
    display: flex;
    flex-wrap: nowrap;
    justify-content: flex-end;
    align-items: center;
    gap: var(--action-inline-gap);
}

.compact-form :deep(.el-button) {
    height: 30px;
}

.result-divider {
    margin: var(--divider-gap-top) 0 var(--divider-gap-bottom);
}

.result-divider :deep(.el-divider__text) {
    background: var(--el-bg-color-overlay);
    padding: 0 10px;
    font-weight: 600;
}

.join-how-group {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
}

.join-how-group :deep(.el-radio-button) {
    width: 100%;
}

.join-how-group :deep(.el-radio-button + .el-radio-button) {
    margin-left: 0;
}

.join-how-group :deep(.el-radio-button__inner) {
    width: 100%;
    text-align: center;
    border-radius: 0 !important;
    border: 1px solid var(--el-border-color) !important;
    box-shadow: none !important;
}

.join-how-group :deep(.el-radio-button:first-child .el-radio-button__inner),
.join-how-group :deep(.el-radio-button:last-child .el-radio-button__inner) {
    border-radius: 0 !important;
}

.join-how-group :deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
    border-color: var(--el-color-primary) !important;
}

.merge-tabs :deep(.el-tabs__header) {
    margin-bottom: var(--tabs-header-gap);
}

.keys-label {
    font-size: 12px;
    color: var(--el-text-color-secondary);
    font-weight: 500;
}

.key-pair-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
}

.key-arrow {
    color: var(--el-text-color-secondary);
    flex: none;
    font-size: 14px;
}

.drop-zone {
    border: 1px dashed var(--el-border-color);
    border-radius: 8px;
    padding: 10px;
    margin: 4px 0 10px;
    background: color-mix(in srgb, var(--el-bg-color-overlay) 86%, var(--el-color-primary) 14%);
    transition: border-color 0.15s, background 0.15s;
    cursor: pointer;
}

.drop-zone.active {
    border-color: var(--el-color-primary);
    background: color-mix(in srgb, var(--el-bg-color-overlay) 74%, var(--el-color-primary) 26%);
}

.drop-main {
    font-size: 13px;
    color: var(--el-text-color-primary);
    font-weight: 600;
}

.drop-sub {
    margin-top: 2px;
    font-size: 12px;
    color: var(--el-text-color-secondary);
}

.path-list-wrap {
    border: 1px solid var(--el-border-color-light);
    border-radius: 8px;
    margin-bottom: 10px;
}

.path-list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--el-text-color-primary);
    border-bottom: 1px solid var(--el-border-color-lighter);
}

.path-list {
    max-height: 120px;
    overflow: auto;
    padding: 4px;
}

.path-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 4px;
}

.path-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--el-text-color-regular);
}
</style>
