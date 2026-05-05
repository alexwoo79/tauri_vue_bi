# BI 分析工具（tauri-vue-bi）

基于 Tauri v2 + Vue 3 + Rust 的桌面 BI 工具，提供数据加载清洗、图表分析、透视分析和甘特图分析能力。

## 技术栈

| 层次 | 技术 |
|---|---|
| 桌面壳 | Tauri v2 |
| 前端 | Vue 3 + Element Plus + ECharts + Pinia + Vue Router 4 (Hash) |
| 后端数据处理 | Rust + Polars |
| Excel I/O | calamine（读取）+ rust_xlsxwriter（导出） |
| 构建工具 | Vite 5 |

## 页面路由

| 页面 | 路由 |
|---|---|
| 数据加载与清洗 | `/load-clean` |
| 图表分析 | `/chart-analysis` |
| Pivot 分析 | `/pivot-analysis` |
| 甘特图分析 | `/gantt-analysis` |

## 项目结构

```text
tauri-vue-bi/
├── src/                          # 前端（Vue 3）
│   ├── App.vue
│   ├── main.ts
│   ├── router/index.ts
│   ├── stores/dataStore.ts
│   ├── components/
│   ├── views/
│   └── utils/
├── src-tauri/                    # 后端（Tauri + Rust）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── icons/
│   │   ├── icon-source.svg       # 图标源文件（用于生成各平台图标）
│   │   ├── icon.ico
│   │   └── icon.icns
│   └── src/
│       ├── main.rs
│       ├── lib.rs                # Tauri 命令注册入口
│       └── commands.rs           # 数据处理实现
├── Makefile
├── package.json
└── README.md
```

## 环境要求

- Node.js >= 20（CI 使用 Node 20）
- Rust toolchain 1.88.0（见 `src-tauri/rust-toolchain.toml`）
- Tauri CLI v2（已在前端 devDependencies 中声明，可通过 npm script 调用）

## 快速开始

```bash
npm ci
```

### 开发模式

```bash
npm run tauri -- dev
# 或
make dev
```

### 构建前端

```bash
npm run build
# 或
make build
```

### 打包桌面应用

```bash
make bundle
```

说明：`make bundle` 会先执行 `make icon`，使用 `src-tauri/icons/icon-source.svg` 重新生成图标，再进行 Tauri build，避免空/损坏图标导致打包失败。

构建产物目录：`src-tauri/target/release/bundle/`

## Makefile 常用命令

```bash
make install       # npm ci
make dev           # tauri dev
make build         # 前端构建（vue-tsc + vite build）
make icon          # 重新生成 tauri 图标
make bundle        # 打包桌面应用
make dmg           # 仅构建 macOS DMG
make test          # Rust + 前端测试
make lint          # ESLint + clippy
make fmt           # rustfmt + prettier（如已安装）
```

## GitHub Actions 发布流程

发布工作流文件：`.github/workflows/release-tauri-vue-bi.yml`

### 触发方式

1. Tag 触发（推荐）

```bash
make release TAG=tauri-vue-bi-v0.1.0
```

这会执行：发布前检查、创建 tag、推送 tag。随后 GitHub Actions 自动触发多平台构建与发布。

2. 手动触发（workflow_dispatch）

- 在 GitHub Actions 页面手动运行 `Release tauri-vue-bi`
- 填写 `release_tag`（建议与 tag 规范一致，例如 `tauri-vue-bi-v0.1.0`）

### 流程说明

- 矩阵平台：`macos-latest`、`ubuntu-22.04`、`windows-latest`
- Node：20
- Rust：stable
- Linux 额外安装 WebKit/GTK 等依赖
- 打包与发布由 `tauri-apps/tauri-action@v0` 完成
- 发布产物包含 macOS / Windows / Linux 安装包

### 发布前建议

```bash
make release-check
```

若只想先本地验证打包：

```bash
make bundle
```

### 常见问题排查

1. 报错 `failed to parse icon ... fill whole buffer`

- 原因：`src-tauri/icons` 下图标文件为空或损坏（尤其是 `icon.ico` / `icon.icns`）
- 处理：

```bash
make icon
git add src-tauri/icons
git commit -m "fix: regenerate tauri icons"
git push
```

2. Tag 未触发工作流

- 确认 tag 符合模式：`tauri-vue-bi-v*`
- 用以下命令推送 tag：

```bash
git push origin tauri-vue-bi-v0.1.0
```

3. 发布失败但本地可构建

- 检查是否已提交最新 `src-tauri/icons` 文件
- 检查 `package-lock.json` 是否存在且与依赖同步
- 重新触发失败平台任务，优先查看该平台日志中的第一条错误

## 后端 Tauri 命令

以下命令在 `src-tauri/src/lib.rs` 注册并暴露给前端：

- `load_file`
- `get_dataframe_info`
- `fetch_chart_data`
- `pivot_data`
- `clean_data`
- `undo_clean`
- `rollback_clean`
- `groupby_agg`
- `fetch_gantt_data`
- `save_file`

## 开发说明

### 新增图表类型

1. 在 `src/utils/chartAdapter.ts` 中扩展图表类型。
2. 在 `buildChartOption` 分支逻辑中补充 option 生成。
3. 在 `src/views/ChartAnalysis.vue` 的图表类型列表中增加选项。

### 新增后端命令

1. 在 `src-tauri/src/commands.rs` 增加 `*_impl` 逻辑。
2. 在 `src-tauri/src/lib.rs` 增加 `#[tauri::command]` 包装函数。
3. 在 `tauri::generate_handler![]` 中注册新命令。
4. 前端通过 `invoke('command_name', params)` 调用。
