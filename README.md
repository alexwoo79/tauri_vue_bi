# tauri-vue-bi/README.md
# BI 分析工具 — Tauri + Vue3 + Rust (DuckDB / Polars) + ECharts

这是对 `bi/` 目录下 Python/Streamlit BI 工具的 **技术栈升级版**，使用以下技术栈重构：

| 层次        | 技术                              |
|------------|-----------------------------------|
| 桌面壳      | [Tauri v2](https://tauri.app/)     |
| 前端 UI     | Vue 3 + Element Plus + ECharts     |
| 数据处理    | Rust (Polars + DuckDB)             |
| 状态管理    | Pinia                              |
| 路由        | Vue Router 4 (Hash 模式)           |
| 构建工具    | Vite 5                             |

---

## 功能对照

| 原 Streamlit 模式       | Tauri 页面路由             |
|------------------------|---------------------------|
| ⬇️ 清洗导出             | `/load-clean`              |
| 📊 图表分析              | `/chart-analysis`          |
| 🔢 Pivot 分析           | `/pivot-analysis`          |
| 甘特图 (独立页面)        | `/gantt-analysis`          |

---

## 项目结构

```
tauri-vue-bi/
├── src-tauri/                    # 🦀 后端：Rust + DuckDB/Polars 核心引擎
│   ├── Cargo.toml                # 依赖配置（duckdb, polars, tokio 等）
│   ├── build.rs                  # Tauri 编译脚本
│   ├── tauri.conf.json           # Tauri 应用配置
│   └── src/
│       ├── main.rs               # 二进制入口
│       ├── lib.rs                # 引擎入口：Tauri 命令注册 + 全局状态
│       └── commands.rs           # 核心数据处理逻辑（load/chart/pivot/clean/groupby/gantt）
│
├── src/                          # 🟩 前端：Vue3 + Element Plus + ECharts
│   ├── main.ts                   # 全局挂载：Element Plus 暗黑主题 + ECharts 按需注册
│   ├── App.vue                   # 主布局：侧边栏菜单 + 路由调度
│   ├── router/index.ts           # Vue Router 路由配置
│   ├── stores/dataStore.ts       # Pinia 全局数据状态
│   ├── components/
│   │   ├── BiChart.vue           # 通用图表组件（vue-echarts 封装）
│   │   └── BiGanttChart.vue      # 高级甘特图组件（ECharts custom series）
│   ├── views/
│   │   ├── LoadClean.vue         # 数据加载与清洗面板
│   │   ├── ChartAnalysis.vue     # 图表分析与 TopN 控制面板
│   │   ├── PivotAnalysis.vue     # 多维透视表操作面板
│   │   └── GanttAnalysis.vue     # 甘特图进度与统计分析面板
│   └── utils/
│       └── chartAdapter.ts       # ECharts option 生成工厂（按图表类型分发）
│
├── index.html                    # Vite HTML 入口
├── vite.config.ts                # Vite 配置（Tauri 开发模式）
├── package.json                  # 前端依赖
└── tsconfig.json                 # TypeScript 配置
```

---

## 快速开始

### 前置要求

- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.77
- [Node.js](https://nodejs.org/) ≥ 18
- [Tauri CLI v2](https://tauri.app/v2/guides/getting-started/setup/)

```bash
# 安装 Tauri CLI
cargo install tauri-cli --version "^2"
```

### 安装依赖

```bash
cd tauri-vue-bi
npm install
```

### 开发模式

```bash
npm run tauri dev
# 等价于：cargo tauri dev
```

Tauri 会自动启动 Vite 开发服务器（端口 1420）和 Rust 后端，并打开原生窗口。

### 生产构建

```bash
npm run tauri build
# 构建产物位于 src-tauri/target/release/bundle/
```

---

## 后端 Tauri 命令说明

| 命令                   | 说明                                   |
|-----------------------|----------------------------------------|
| `load_file`           | 加载 CSV / Excel 文件到全局 DataFrame   |
| `get_dataframe_info`  | 获取当前 DataFrame 的列信息和前 N 行     |
| `fetch_chart_data`    | 图表数据：排序、TopN                     |
| `pivot_data`          | 多维透视（行分组、列分组、聚合）          |
| `clean_data`          | 数据清洗流水线                           |
| `groupby_agg`         | 分组聚合（groupby + agg）               |
| `fetch_gantt_data`    | 甘特图数据提取                           |

---

## 开发指南

### 添加新图表类型

1. 在 `src/utils/chartAdapter.ts` 的 `ChartType` 联合类型中添加新类型名称
2. 在 `buildChartOption` 的 `switch` 语句中添加对应 case
3. 在 `src/views/ChartAnalysis.vue` 的 `chartTypeOptions` 数组中添加选项

### 添加新的后端命令

1. 在 `src-tauri/src/commands.rs` 中实现 `*_impl` 纯函数
2. 在 `src-tauri/src/lib.rs` 中添加对应的 `#[tauri::command]` 函数（调用 impl）
3. 将新命令添加到 `run()` 函数的 `tauri::generate_handler![]` 宏中
4. 在前端使用 `invoke('command_name', { ...params })` 调用
# tauri_vue_bi
