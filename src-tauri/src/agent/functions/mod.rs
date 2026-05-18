// src-tauri/src/agent/functions/mod.rs
//
// 函数模块入口 - 对标 Python Data-Analysis-Agent 的 Function/
//
// 包含四个主要子模块：
// - charts    : 图表生成
// - analyze   : 数据分析（十分位分析、决策树、K-Means等）
// - clean     : 数据清洗（数据概况、缺失值处理、缩尾、截断）
// - output    : 输出导出（Excel、报告、PPT）

pub mod charts;
pub mod analyze;
pub mod clean;
pub mod output;

// 重新导出所有子模块的内容
pub use charts::*;
pub use analyze::*;
pub use clean::*;
pub use output::*;
