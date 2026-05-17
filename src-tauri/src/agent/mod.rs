// src-tauri/src/agent/mod.rs
//
// Agent 核心模块 - 业务分析智能体
//
// 文件结构（对应 Python Agent）：
// - session.rs          → 会话管理
// - state_machine.rs    → Agent 核心逻辑（对应 agent.py）
// - prompts.rs          → 提示词管理（对应 prompts.py）
// - tools_schema.rs     → 工具 Schema 定义（对应 tools_schema.py）
// - mcp_manager.rs      → MCP 管理器（对应 mcp_manager.py，可选）
// - tools/              → 工具实现
//   - data_tools.rs     → 数据工具（对应 tools_data.py）
//   - export_tools.rs   → 导出工具（对应 tools_export.py）
//   - chart_engine.rs   → 图表引擎
//   - chart_tools.rs    → 图表工具
//   - color_schemes.rs  → 配色方案
//   - plotly_tools.rs   → Plotly 工具

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
