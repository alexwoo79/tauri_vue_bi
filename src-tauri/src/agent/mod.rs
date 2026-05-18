// src-tauri/src/agent/mod.rs
//
// Agent 核心模块 - 业务分析智能体
//
// 文件结构（严格对标 Python Data-Analysis-Agent）：
// - session.rs          → 会话管理
// - state_machine.rs    → Agent 核心逻辑（对应 agent.py）
// - prompts.rs          → 提示词管理（对应 prompts.py）
// - tools_schema.rs     → 工具 Schema 定义（对应 tools_schema.py）
// - functions/          → 函数实现（对应 functions/）
//   - base.rs           → 基础类型定义（FieldMapping, ChartOptions, ChartResult）
//   - chart_generate.rs → 图表生成入口
//   - registry.rs       → 图表注册表
//   - color_schemes.rs  → 配色方案
//   - charts/           → 各图表类型实现
//     - bar_chart.rs
//     - line_chart.rs
//     - pie_chart.rs
//     - ...

pub mod session;
pub mod state_machine;
pub mod prompts;        // ✅ 新增
pub mod tools_schema;   // ✅ 新增
pub mod functions;      // ✅ 新增：图表函数模块
pub mod dataops;         // ✅ 数据和导出工具
pub mod llm;            // ✅ 新增：LLM 模块
pub mod api;            // ✅ 新增：API 模块

// pub mod mcp_manager; // ⏸️ 可选，后续添加

pub use session::*;
pub use state_machine::*;
pub use prompts::*;
pub use tools_schema::*;
pub use functions::*;
pub use llm::*;
pub use api::*;