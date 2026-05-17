// src-tauri/src/agent/tools/mod.rs
//
// Agent 工具模块
//
// 包含所有可供 LLM 调用的工具函数：
// - data_tools: 数据查询和分析工具
// - export_tools: 导出和报告生成工具
// - chart_tools: ECharts 图表生成工具（旧版）
// - chart_engine: Plotly.rs 图表引擎（新版，参考 Python Agent）
// - color_schemes: 配色方案系统
// - plotly_tools: Plotly.rs 原型测试（已废弃，合并到 chart_engine）
// - mcp_tools: MCP (Model Context Protocol) 工具集成

pub mod chart_engine;   // 完整的 Plotly.rs 图表引擎
pub mod chart_tools;    // ECharts 图表工具（向后兼容）
pub mod color_schemes;  // 配色方案系统
pub mod data_tools;
pub mod export_tools;

pub use chart_engine::*;
pub use chart_tools::*;
pub use color_schemes::*;
pub use data_tools::*;
pub use export_tools::*;
