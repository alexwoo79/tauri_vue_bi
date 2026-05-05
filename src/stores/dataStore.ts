// src/stores/dataStore.ts
// Pinia 状态管理 — 全局数据状态 (Global Data State)
//
// 存储当前加载的 DataFrame 信息，在各个 View 之间共享。

import { defineStore } from 'pinia'
import { shallowRef, ref, computed } from 'vue'
import type { ColumnInfo, ChartPayload } from '../utils/chartAdapter'
import { normalizeThemeName } from '../utils/echartsTheme'

export const useDataStore = defineStore('data', () => {
  // 当前加载的数据预览（前 100 行）
  // 使用 shallowRef 避免深度响应造成的内存开销
  // 只有顶级对象变化时才触发更新，而不跟踪数组内部的行对象
  const payload = shallowRef<ChartPayload | null>(null)

  // 当前 ECharts 主题名称
  const currentTheme = ref<string>('v5')

  // 是否已加载数据
  const hasData = computed(() => payload.value !== null && payload.value.total_rows > 0)

  // 所有列信息
  const columns = computed<ColumnInfo[]>(() => payload.value?.columns ?? [])

  // 所有列名
  const columnNames = computed<string[]>(() => columns.value.map((c) => c.name))

  // 数值列（用于图表 Y 轴等）
  // 兼容多种后端 dtype 命名：Polars / Pandas / 自定义 "Number" 等。
  const NUMERIC_DTYPES = new Set([
    'Int8', 'Int16', 'Int32', 'Int64',
    'UInt8', 'UInt16', 'UInt32', 'UInt64',
    'Float16', 'Float32', 'Float64',
    'Decimal', 'Number',
    'int8', 'int16', 'int32', 'int64',
    'uint8', 'uint16', 'uint32', 'uint64',
    'float16', 'float32', 'float64',
    'decimal', 'number',
    'i8', 'i16', 'i32', 'i64',
    'u8', 'u16', 'u32', 'u64',
    'f16', 'f32', 'f64',
  ])

  function isNumericDtype(dtype: string): boolean {
    const d = String(dtype ?? '').trim()
    if (!d) return false
    if (NUMERIC_DTYPES.has(d)) return true

    const lower = d.toLowerCase()
    if (NUMERIC_DTYPES.has(lower)) return true

    // 兜底：处理如 Int128 / Float128 / Decimal(10,2) / Nullable(Int64) 等格式
    return /(int|uint|float|double|decimal|number)/.test(lower)
  }
  const numericColumns = computed<string[]>(() =>
    columns.value
      .filter((c) => isNumericDtype(c.dtype))
      .map((c) => c.name)
  )

  // 字符串/分类列
  // Polars formats these as "String", "Utf8", "Categorical", "Boolean"
  const CATEGORICAL_DTYPES = new Set(['String', 'Utf8', 'Categorical', 'Boolean'])
  const categoricalColumns = computed<string[]>(() =>
    columns.value
      .filter((c) => CATEGORICAL_DTYPES.has(c.dtype))
      .map((c) => c.name)
  )

  // 日期/时间列
  // Polars formats these as "Date", "Datetime(...)", "Duration(...)", "Time"
  const dateColumns = computed<string[]>(() =>
    columns.value
      .filter((c) => {
        const d = String(c.dtype ?? '').trim().toLowerCase()
        return d === 'date' || d === 'time' || d.startsWith('datetime') || d.startsWith('duration')
      })
      .map((c) => c.name)
  )

  function setPayload(p: ChartPayload | null) {
    payload.value = p
  }

  function setTheme(theme: string) {
    currentTheme.value = normalizeThemeName(theme)
  }

  function clear() {
    payload.value = null
  }

  return {
    payload,
    hasData,
    columns,
    columnNames,
    numericColumns,
    categoricalColumns,
    dateColumns,
    currentTheme,
    setPayload,
    setTheme,
    clear,
  }
})
