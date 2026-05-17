// src-tauri/src/agent/tools/mod.rs
//
// Agent 工具模块
//
// 包含所有可供 LLM 调用的工具函数：
// - data_tools: 数据查询和分析工具
// - export_tools: 导出和报告生成工具
// - chart_engine: Plotly.rs 图表引擎（统一使用）
// - color_schemes: 配色方案系统

pub mod chart_engine;   // ✅ 完整的 Plotly.rs 图表引擎（统一使用）
pub mod color_schemes;  // 配色方案系统
pub mod data_tools;
pub mod export_tools;

pub use chart_engine::*;  // ✅ 只导出 chart_engine
pub use color_schemes::*;
pub use data_tools::*;
pub use export_tools::*;
