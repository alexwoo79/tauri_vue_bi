// src/composables/useDatasetActions.ts
//
// 数据集管理操作 Composable
//
// 封装了 list_datasets / switch_dataset / delete_datasets / save_current_dataset
// 等跨多个页面重复使用的 Tauri IPC 调用，以及对应的 loading 状态和错误处理。
//
// 用法：
//   const { datasets, loadDatasets, switchDataset, deleteDatasets } = useDatasetActions()

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import type { DatasetMeta, ChartPayload } from '../utils/chartAdapter'
import { useDataStore } from '../stores/dataStore'

export function useDatasetActions() {
  const dataStore = useDataStore()
  const switching = ref(false)

  /** 从后端刷新数据集列表，并同步到 store */
  async function loadDatasets() {
    try {
      const result: { ok: boolean; data?: DatasetMeta[]; error?: string } =
        await invoke('list_datasets')
      if (result.ok && result.data) {
        dataStore.setDatasets(result.data)
      }
    } catch (e) {
      console.error('list_datasets failed:', e)
    }
  }

  /** 切换当前活跃数据集 */
  async function switchDataset(datasetId: string): Promise<boolean> {
    switching.value = true
    try {
      const result: { ok: boolean; data?: ChartPayload; error?: string } = await invoke(
        'switch_dataset',
        { datasetId }
      )
      if (result.ok && result.data) {
        dataStore.setPayload(result.data)
        ElMessage.success('数据集已切换')
        return true
      } else {
        ElMessage.error(result.error ?? '切换失败')
        return false
      }
    } catch (e) {
      ElMessage.error(String(e))
      return false
    } finally {
      switching.value = false
    }
  }

  /** 删除一个或多个数据集，同步 store 并返回更新后的列表 */
  async function deleteDatasets(ids: string[]): Promise<DatasetMeta[]> {
    try {
      const result: { ok: boolean; data?: DatasetMeta[]; error?: string } = await invoke(
        'delete_datasets',
        { datasetIds: ids }
      )
      if (result.ok && result.data) {
        dataStore.setDatasets(result.data)
        return result.data
      } else {
        ElMessage.error(result.error ?? '删除失败')
        return dataStore.datasets
      }
    } catch (e) {
      ElMessage.error(String(e))
      return dataStore.datasets
    }
  }

  /** 将当前 DataFrame 另存为新数据集 */
  async function saveCurrentDataset(name: string, source?: string): Promise<DatasetMeta | null> {
    try {
      const result: { ok: boolean; data?: DatasetMeta; error?: string } = await invoke(
        'save_current_dataset',
        { name, source: source ?? 'manual_save' }
      )
      if (result.ok && result.data) {
        await loadDatasets()
        return result.data
      } else {
        ElMessage.error(result.error ?? '保存失败')
        return null
      }
    } catch (e) {
      ElMessage.error(String(e))
      return null
    }
  }

  return {
    switching,
    loadDatasets,
    switchDataset,
    deleteDatasets,
    saveCurrentDataset,
  }
}
