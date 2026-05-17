# Rust Agent 文件结构对齐报告 - 2026-05-17

## 📅 日期: 2026-05-17

---

## 🎯 目标

将 Rust Agent 的文件结构与 Python Agent 完全对齐，便于后期一一对照和维护。

---

## 📊 文件结构对比

### Python Agent 目录结构
```
Data-Analysis-Agent/agent/
├── __init__.py              # 模块初始化
├── agent.py                 # Agent 核心逻辑（34.0KB）
├── mcp_manager.py           # MCP 管理器（19.2KB）
├── prompts.py               # 提示词管理（18.2KB）
├── tools_data.py            # 数据工具（11.6KB）
├── tools_export.py          # 导出工具（18.5KB）
└── tools_schema.py          # 工具 Schema 定义（26.7KB）
```

### Rust Agent 目录结构（重构后）
```
src-tauri/src/agent/
├── mod.rs                   # ✅ 模块导出
├── session.rs               # ✅ 会话管理（5.5KB）
├── state_machine.rs         # ✅ Agent 核心逻辑（25.2KB）→ 对应 agent.py
├── prompts.rs               # ✅ 提示词管理（新建）→ 对应 prompts.py
├── tools_schema.rs          # ✅ 工具 Schema 定义（新建）→ 对应 tools_schema.py
├── mcp_manager.rs           # ⏸️ 可选（后续添加）→ 对应 mcp_manager.py
└── tools/                   # ✅ 工具实现目录
    ├── mod.rs
    ├── data_tools.rs        # ✅ 数据工具（从 tools_data.rs 移动）→ 对应 tools_data.py
    ├── export_tools.rs      # ✅ 导出工具（从 tools_export.rs 移动）→ 对应 tools_export.py
    ├── chart_engine.rs      # ✅ 图表引擎（16+ 种图表）
    ├── chart_tools.rs       # ✅ 图表工具（向后兼容）
    ├── color_schemes.rs     # ✅ 配色方案系统
    └── plotly_tools.rs      # ✅ Plotly 工具
```

---

## ✅ 已完成的重构工作

### 1. 删除冗余文件
- ❌ **删除** `agent.rs` - 未使用的副本文件
- ✅ **保留** [state_machine.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/state_machine.rs) - 作为主要的 Agent 逻辑文件

### 2. 移动工具文件到 tools/ 目录
```bash
mv src/agent/tools_data.rs src/agent/tools/data_tools.rs
mv src/agent/tools_export.rs src/agent/tools/export_tools.rs
```

**影响**:
- ✅ 与 Python 的 `tools_data.py` 和 `tools_export.py` 对齐
- ✅ 所有工具实现在统一的 `tools/` 目录下
- ✅ [tools/mod.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools/mod.rs) 已正确导出

### 3. 创建 prompts.rs（对应 prompts.py）

**文件**: `src-tauri/src/agent/prompts.rs`

**功能**:
- ✅ `get_command_hint()` - 命令提示映射（对应 Python 的 COMMAND_HINTS）
- ✅ `build_chart_guide()` - 动态生成图表指南（对应 [_build_chart_guide()](file:///Users/alex/Documents/github/tauri-vue-bi/Data-Analysis-Agent/agent/prompts.py#L31-L51)）
- ✅ `get_all_chart_ids()` - 获取所有图表 ID 列表
- ✅ `build_analyze_guide()` - 构建分析指南（对应 [_build_analyze_guide()](file:///Users/alex/Documents/github/tauri-vue-bi/Data-Analysis-Agent/agent/prompts.py#L23-L28)）

**关键改进**:
```rust
// ✅ 动态从 ChartRegistry 生成图表指南
pub fn build_chart_guide() -> String {
    let registry = ChartRegistry::new();
    let charts = registry.list_charts();
    
    // 按类别分组并格式化输出
    // ...
}
```

**对比 Python**:
```python
# Python: 从注册表动态生成
def _build_chart_guide() -> tuple:
    from charts.registry import REGISTRY
    lines, ids = [], []
    for c in REGISTRY:
        lines.append(f"  {c.chart_id:<35} → {c.desc[:80]}")
    return "\n".join(lines), ", ".join(ids)
```

### 4. 创建 tools_schema.rs（对应 tools_schema.py）

**文件**: `src-tauri/src/agent/tools_schema.rs`

**功能**:
- ✅ `get_agent_tools()` - 返回所有工具的 JSON Schema 列表
- ✅ 包含 11 个工具定义：
  1. `get_schema` - 获取数据源结构
  2. `create_analysis_table` - 创建分析表
  3. `query_data` - 执行 SQL 查询
  4. `run_analysis` - 运行统计分析
  5. `profile_data` - 数据概况分析
  6. `clean_data` - 数据清洗
  7. `generate_chart` - 生成图表
  8. `propose_excel_export` - Excel 导出提议
  9. `propose_ppt_outline` - PPT 大纲提议
  10. `propose_report_outline` - 报告大纲提议
  11. `propose_dashboard_outline` - Dashboard 大纲提议

**对比 Python**:
```python
# Python: AGENT_TOOLS 列表
AGENT_TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "get_schema",
            "description": "...",
            "parameters": {...}
        }
    },
    # ... 更多工具
]
```

```rust
// Rust: get_agent_tools() 函数
pub fn get_agent_tools() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "get_schema",
                "description": "...",
                "parameters": {...}
            }
        }),
        // ... 更多工具
    ]
}
```

### 5. 更新 state_machine.rs

**修改内容**:
1. ✅ 导入 `prompts` 模块
2. ✅ 修改 [build_system_prompt()](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/commands/agent_chat.rs#L256-L272) 使用动态图表指南
3. ✅ 删除旧的 `get_command_hint()` 函数，改用 `prompts::get_command_hint()`

**关键代码**:
```rust
use crate::agent::prompts;  // ✅ 新增导入

fn build_system_prompt(command: Option<&str>) -> String {
    let base = include_str!("prompts/system_prompt.md");
    
    // ✅ 动态生成图表指南并替换占位符
    let chart_guide = prompts::build_chart_guide();
    let base_with_charts = base.replace("{{CHART_GUIDE}}", &chart_guide);
    
    if let Some(cmd) = command {
        if let Some(hint) = prompts::get_command_hint(cmd) {
            return format!("{}\n\n[ACTIVE COMMAND: /{}]\n{}", base_with_charts, cmd, hint);
        }
    }
    
    base_with_charts
}
```

### 6. 更新 agent/mod.rs

**修改内容**:
```rust
pub mod session;
pub mod state_machine;
pub mod prompts;        // ✅ 新增
pub mod tools_schema;   // ✅ 新增
pub mod tools;

// pub mod mcp_manager; // ⏸️ 可选，后续添加

pub use session::*;
pub use state_machine::*;
pub use prompts::*;
pub use tools_schema::*;
```

---

## 📋 文件对应关系总览

| Python 文件 | Rust 文件 | 状态 | 说明 |
|------------|----------|------|------|
| `agent.py` | [state_machine.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/state_machine.rs) | ✅ 对齐 | Agent 核心逻辑 |
| `prompts.py` | [prompts.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/prompts.rs) | ✅ 对齐 | 提示词管理 |
| `tools_schema.py` | [tools_schema.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools_schema.rs) | ✅ 对齐 | 工具 Schema 定义 |
| `tools_data.py` | `tools/data_tools.rs` | ✅ 对齐 | 数据工具 |
| `tools_export.py` | `tools/export_tools.rs` | ✅ 对齐 | 导出工具 |
| `mcp_manager.py` | `mcp_manager.rs` | ⏸️ 待实现 | MCP 管理器（可选） |
| - | [session.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/session.rs) | ✅ Rust 特有 | 会话管理 |
| - | `tools/chart_engine.rs` | ✅ Rust 特有 | 图表引擎 |
| - | `tools/chart_tools.rs` | ✅ Rust 特有 | 图表工具 |
| - | `tools/color_schemes.rs` | ✅ Rust 特有 | 配色方案 |
| - | `tools/plotly_tools.rs` | ✅ Rust 特有 | Plotly 工具 |

---

## 🎯 优势

### 1. 易于对照维护
- ✅ 文件名一一对应，方便查找
- ✅ 功能模块清晰分离
- ✅ 便于从 Python 迁移新功能

### 2. 动态生成 vs 硬编码
- ✅ **Python**: 从注册表动态生成图表指南
- ✅ **Rust**: 同样从 [ChartRegistry](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools/chart_engine.rs#L136-L150) 动态生成
- ✅ 避免硬编码，保持同步

### 3. 工具 Schema 统一管理
- ✅ **Python**: [tools_schema.py](file:///Users/alex/Documents/github/tauri-vue-bi/Data-Analysis-Agent/agent/tools_schema.py) 集中定义
- ✅ **Rust**: [tools_schema.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools_schema.rs) 集中定义
- ✅ 便于维护和扩展

### 4. 模块化设计
- ✅ 提示词、工具、Schema 分离
- ✅ 每个模块职责单一
- ✅ 便于单元测试

---

## 🚀 下一步行动

### 高优先级
1. **验证编译** - 确保所有修改不影响编译
   ```bash
   cd /Users/alex/Documents/github/tauri-vue-bi/src-tauri
   cargo check --lib
   ```

2. **测试动态图表指南** - 验证 [build_chart_guide()](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/prompts.rs#L119-L149) 是否正确生成
   ```rust
   #[test]
   fn test_build_chart_guide() {
       let guide = prompts::build_chart_guide();
       assert!(guide.contains("Bar_Chart"));
       assert!(guide.contains("Line_Chart"));
   }
   ```

3. **测试工具 Schema** - 验证 [get_agent_tools()](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools_schema.rs#L11-L323) 返回正确的 JSON
   ```rust
   #[test]
   fn test_get_agent_tools() {
       let tools = tools_schema::get_agent_tools();
       assert_eq!(tools.len(), 11);
   }
   ```

### 中优先级
4. **实现 MCP Manager**（可选）
   - 如果计划支持 Model Context Protocol
   - 对应 Python 的 [mcp_manager.py](file:///Users/alex/Documents/github/tauri-vue-bi/Data-Analysis-Agent/agent/mcp_manager.py)

5. **完善 prompts.rs**
   - 添加更多命令提示
   - 实现动态分析指南（从注册表读取）

### 低优先级
6. **添加单元测试**
   - 为 [prompts.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/prompts.rs) 添加测试
   - 为 [tools_schema.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools_schema.rs) 添加测试

7. **文档完善**
   - 为每个模块添加详细的 doc comments
   - 创建模块间关系图

---

## 📝 注意事项

### 1. system_prompt.md 中的占位符
- ✅ 已将硬编码的图表列表替换为 `{{CHART_GUIDE}}`
- ✅ 在运行时通过 [build_system_prompt()](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/commands/agent_chat.rs#L256-L272) 动态替换
- ⚠️ 确保占位符名称一致

### 2. 工具 Schema 的维护
- ✅ 新增工具时，需要在 [tools_schema.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/tools_schema.rs) 中添加对应的 JSON Schema
- ✅ 保持与 Python 版本的 Schema 一致
- ⚠️ 注意参数类型和必填字段的定义

### 3. 命令提示的同步
- ✅ [prompts.rs](file:///Users/alex/Documents/github/tauri-vue-bi/src-tauri/src/agent/prompts.rs) 中的 `get_command_hint()` 应与 Python 的 [COMMAND_HINTS](file:///Users/alex/Documents/github/tauri-vue-bi/Data-Analysis-Agent/agent/prompts.py#L103-L311) 保持一致
- ⚠️ 当 Python 版本更新命令提示时，需要同步更新 Rust 版本

---

## 📊 重构前后对比

### 重构前
```
❌ agent.rs (未使用)
❌ tools_data.rs (在 agent/ 根目录)
❌ tools_export.rs (在 agent/ 根目录)
❌ 缺少 prompts.rs
❌ 缺少 tools_schema.rs
❌ 硬编码的图表列表
```

### 重构后
```
✅ 删除 agent.rs
✅ tools/data_tools.rs (在 tools/ 目录)
✅ tools/export_tools.rs (在 tools/ 目录)
✅ 创建 prompts.rs (动态生成图表指南)
✅ 创建 tools_schema.rs (工具 Schema 定义)
✅ 动态生成图表列表（从 ChartRegistry）
```

---

**最后更新**: 2026-05-17  
**状态**: ✅ **文件结构已完全对齐 Python Agent**  
**预计影响**: 大幅提升代码可维护性和对照便利性
