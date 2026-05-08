# 重构工作计划 — tauri_vue_bi

> 版本：v1.0 | 日期：2026-05-08

---

## 一、现状与问题

| 文件 | 行数 | 问题 |
|---|---|---|
| `src-tauri/src/commands.rs` | ~1305 | 5 大领域混在一个文件 |
| `src-tauri/src/lib.rs` | ~970 | 类型/状态/工具/命令全混 |
| `src/views/LoadClean.vue` | ~1200 | 拖拽区、清洗面板、数据列表全混 |
| `src/views/ChartAnalysis.vue` | ~570 | 图类选择、轴配置、排序/过滤混 |
| `src/utils/chartAdapter.ts` | ~460 | 8 种图形同在一个文件 |

---

## 二、后端拆分计划（Rust）

### 目标目录结构

```
src-tauri/src/
├── main.rs               (不变)
├── lib.rs                ← 精简为：模块声明 + 状态引用 + run()
├── types.rs              (新) 共享数据类型
├── state.rs              (新) 全局静态状态 + Dataset 持久化
├── df_util.rs            (新) df_to_payload / dtype 推断工具
└── commands/
    ├── mod.rs            (新) 统一 re-export
    ├── loader.rs         (新) load_file 命令 + CSV/Excel 加载实现
    ├── chart.rs          (新) fetch_chart_data 命令 + 实现
    ├── clean.rs          (新) clean_data / undo / rollback 命令 + 实现
    ├── pivot.rs          (新) pivot_data 命令 + 实现
    ├── melt.rs           (新) melt_data 命令 + 实现
    ├── groupby.rs        (新) groupby_agg 命令 + 实现
    ├── gantt.rs          (新) fetch_gantt_data 命令 + 实现
    ├── save.rs           (新) save_file 命令 + CSV/XLSX 实现
    └── dataset.rs        (新) list/switch/save/delete dataset 命令
```

### 模块职责边界

| 模块 | 职责 | 来源行范围 |
|---|---|---|
| `types.rs` | 纯数据结构（ApiResult, ChartPayload, DatasetMeta…），无业务逻辑 | lib.rs 50–115 |
| `state.rs` | 5 个全局静态变量 + persist/load/register/sync 函数 | lib.rs 38–300 |
| `df_util.rs` | `df_to_payload`, `infer_payload_dtype`, `series_value_to_json` | lib.rs 307–473 |
| `commands/loader.rs` | CSV/Excel 加载全套逻辑 + `load_file` Tauri 命令 | commands.rs 52–600 |
| `commands/clean.rs` | 清洗 7 步流水线 + undo/rollback + `build_row_filter_mask` | commands.rs 944–1245 |
| `commands/chart.rs` | 列选、排序、TopN + `fetch_chart_data` 命令 | commands.rs 739–793 |
| `commands/pivot.rs` | pivot_data_impl + `pivot_data` 命令 | commands.rs 800–860 |
| `commands/melt.rs` | melt_data_impl + `melt_data` 命令 | commands.rs 862–935 |
| `commands/groupby.rs` | groupby_agg_impl + `groupby_agg` 命令 | commands.rs 1251–1303 |
| `commands/gantt.rs` | Gantt 列筛选 + `fetch_gantt_data` 命令 | lib.rs 786–828 |
| `commands/save.rs` | CSV/XLSX 写出 + `save_file` 命令 | commands.rs 604–720 |
| `commands/dataset.rs` | list/switch/save/delete + `get_dataframe_info` 命令 | lib.rs 839–932 |

### 精简后 lib.rs 骨架

```rust
pub mod types;
pub mod state;
pub mod df_util;
pub mod commands;

use state::*;
use crate::commands::*;

pub fn run() {
    if let Err(e) = state::load_persisted_dataset_registry() { ... }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            loader::load_file,
            dataset::get_dataframe_info,
            chart::fetch_chart_data,
            pivot::pivot_data,
            melt::melt_data,
            clean::clean_data,
            clean::undo_clean,
            clean::rollback_clean,
            groupby::groupby_agg,
            gantt::fetch_gantt_data,
            save::save_file,
            dataset::list_datasets,
            dataset::switch_dataset,
            dataset::save_current_dataset,
            dataset::delete_datasets,
        ])
        .run(...)
}
```

---

## 三、前端拆分计划（Vue / TypeScript）

### 目标目录结构

```
src/
├── App.vue                       (保持精简：导航 + 布局)
├── router/index.ts
├── stores/
│   ├── dataStore.ts              (不变)
│   └── uiStore.ts                (新) sidebar 宽度、主题等全局 UI 状态
├── types/
│   └── chart.ts                  (新) 从 chartAdapter.ts 抽出的纯接口/类型
├── utils/
│   ├── chartAdapter.ts           (精简：只保留 buildChartOption 主入口)
│   ├── charts/                   (新目录)
│   │   ├── bar.ts
│   │   ├── line.ts
│   │   ├── scatter.ts
│   │   ├── pie.ts
│   │   ├── heatmap.ts
│   │   ├── boxplot.ts
│   │   └── index.ts              (re-export)
│   └── echartsTheme.ts
├── composables/                  (新目录)
│   ├── useResize.ts              (4 个页面共用的侧边栏拖拽逻辑)
│   ├── useDatasetActions.ts      (list/switch/delete dataset IPC 调用)
│   └── useExport.ts              (save_file IPC)
├── components/
│   ├── BiChart.vue
│   ├── BiGanttChart.vue
│   ├── SidebarResizable.vue      (新) 可复用拖拽侧边栏容器
│   └── ParamCard.vue             (新) 可折叠参数卡片
└── views/
    ├── LoadClean/
    │   ├── index.vue             (主视图，组装子组件)
    │   ├── DropZone.vue          (拖拽上传区)
    │   ├── CleanPanel.vue        (清洗参数 7 区块)
    │   └── DatasetList.vue       (数据集列表 + 删除)
    ├── ChartAnalysis/
    │   ├── index.vue
    │   ├── ChartTypeSelector.vue (4×n 图表类型矩阵)
    │   ├── AxisConfig.vue        (X/Y 轴、颜色分组、双轴)
    │   └── FilterConfig.vue      (排序、TopN)
    ├── PivotAnalysis/
    │   ├── index.vue
    │   ├── PivotForm.vue
    │   └── MeltForm.vue
    └── GanttAnalysis/
        ├── index.vue
        └── GanttConfig.vue
```

### 优先级排序

| 优先级 | 任务 | 收益 |
|---|---|---|
| ① | 抽取 `useResize.ts` | 消除 4 个页面重复的 startResize 逻辑 |
| ② | 抽取 `useDatasetActions.ts` | 消除 IPC 调用重复代码 |
| ③ | `LoadClean` 拆 3 个子组件 | 主视图从 ~1200 行降至 ~300 行 |
| ④ | `charts/` 目录拆图形构建函数 | 新增图形只需加一个文件 |
| ⑤ | `ChartAnalysis` 拆子组件 | 参数区各自独立维护 |

---

## 四、新增功能扩展操作手册

### 场景 A：新增数据表合并（JOIN / CONCAT）

#### 后端（Rust）

1. 新建 `src-tauri/src/commands/merge.rs`，实现 `merge_data_impl`

```rust
// src-tauri/src/commands/merge.rs
use anyhow::{bail, Result};
use polars::prelude::*;

pub fn merge_data_impl(
    left: &DataFrame,
    right: &DataFrame,
    join_type: &str,
    on: &[String],
) -> Result<DataFrame> {
    let jt = match join_type {
        "inner" => JoinType::Inner,
        "left"  => JoinType::Left,
        "outer" => JoinType::Full { coalesce: true },
        other   => bail!("Unknown join type: {other}"),
    };
    let on_exprs: Vec<_> = on.iter().map(|c| col(c.as_str())).collect();
    left.clone().lazy()
        .join(right.clone().lazy(), &on_exprs, &on_exprs, JoinArgs::new(jt))
        .collect()
        .map_err(|e| anyhow::anyhow!("{e}"))
}

// Tauri 命令 handler —— 在 lib.rs 注册
#[tauri::command]
pub async fn merge_data(...) -> ApiResult<ChartPayload> { ... }
```

2. 在 `commands/mod.rs` 追加：`pub mod merge;`  
3. 在 `lib.rs` 的 `generate_handler!` 追加 `merge::merge_data`  
4. 运行 `cargo check` 验证

#### 前端（Vue）

1. 新建 `src/views/MergeAnalysis/index.vue`，复用 `useResize` / `useDatasetActions`
2. `src/router/index.ts` 追加路由
3. `src/App.vue` 导航菜单追加 `<el-menu-item>`

---

### 场景 B：新增一种 ECharts 图形（以漏斗图为例）

**共 5 步，只需改前端，无需动 Rust。**

#### Step 1 — 追加类型（`src/types/chart.ts`）

```ts
export type ChartType =
  | 'bar_chart' | 'line_chart' | ...
  | 'funnel_chart'  // ← 新增
```

#### Step 2 — 新建图形构建文件（`src/utils/charts/funnel.ts`）

```ts
import type { EChartsOption } from 'echarts'
import type { RowMap } from '../chartAdapter'

export function buildFunnelOption(
  rows: RowMap[], nameCol: string, valueCol: string, title?: string
): EChartsOption {
  return {
    title: title ? { text: title } : undefined,
    tooltip: { trigger: 'item' },
    series: [{
      type: 'funnel',
      data: rows.map(r => ({
        name: String(r[nameCol] ?? ''),
        value: Number(r[valueCol] ?? 0),
      })),
    }],
  }
}
```

#### Step 3 — 注册到 re-export（`src/utils/charts/index.ts`）

```ts
export { buildFunnelOption } from './funnel'
```

#### Step 4 — 接入主入口 switch（`src/utils/chartAdapter.ts`）

```ts
import { buildFunnelOption } from './charts'

case 'funnel_chart':
  return buildFunnelOption(rows, xCol, primaryY, title)
```

#### Step 5 — 添加按钮（`ChartTypeSelector.vue`）

```ts
{ value: 'funnel_chart', icon: '⬇', label: '漏斗图' }
```

---

## 五、重构进度追踪

### 后端

- [x] 工作计划文档
- [x] `types.rs`
- [x] `state.rs`
- [x] `df_util.rs`
- [x] `commands/loader.rs`
- [x] `commands/clean.rs`
- [x] `commands/chart.rs`
- [x] `commands/save.rs`
- [x] `commands/pivot.rs`
- [x] `commands/melt.rs`
- [x] `commands/groupby.rs`
- [x] `commands/gantt.rs`
- [x] `commands/dataset.rs`
- [x] `commands/mod.rs`
- [x] `lib.rs` 瘦身
- [x] 旧 `commands.rs` 删除
- [x] `cargo check` 通过

### 前端

- [ ] `useResize.ts`
- [ ] `useDatasetActions.ts`
- [ ] `LoadClean/` 拆子组件
- [ ] `charts/` 目录拆图形函数
- [ ] `ChartAnalysis/` 拆子组件
- [ ] `PivotAnalysis/` MeltForm 独立
- [ ] `GanttAnalysis/` 拆子组件

---

## 六、工程规范约定

### Rust 命令编写规范

每个 `commands/xxx.rs` 文件结构如下：

```rust
// src-tauri/src/commands/xxx.rs
use anyhow::{bail, Result};
use polars::prelude::*;
use crate::types::*;
use crate::state::*;
use crate::df_util::df_to_payload;

// 1. 纯逻辑实现（无 Tauri 依赖）
pub fn xxx_impl(df: &DataFrame, ...) -> Result<DataFrame> { ... }

// 2. Tauri handler（调用 impl，调用 state，返回 ApiResult）
#[tauri::command]
pub async fn xxx(...) -> ApiResult<ChartPayload> {
    let df = take_df!();
    match xxx_impl(&df, ...) {
        Ok(result_df) => match df_to_payload(&result_df, Some(CHART_LIMIT)) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}
```

### 前端 Composable 编写规范

```ts
// src/composables/useXxx.ts
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

export function useXxx() {
  const loading = ref(false)

  async function doSomething() {
    loading.value = true
    try {
      const result = await invoke('xxx_command', { ... })
      // handle result
    } catch (e) {
      ElMessage.error(String(e))
    } finally {
      loading.value = false
    }
  }

  return { loading, doSomething }
}
```
