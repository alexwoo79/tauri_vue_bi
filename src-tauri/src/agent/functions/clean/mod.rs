// src-tauri/src/agent/functions/clean/mod.rs
//
// 数据清洗模块 - 对标 Python Data-Analysis-Agent 的 Function/Clean/
//
// 包含：
// - data_profile.rs   : 数据概况分析
// - missing_handler.rs: 缺失值处理
// - winsorize.rs      : 缩尾处理
// - trimming.rs       : 截断处理

pub mod data_profile;
pub mod missing_handler;
pub mod winsorize;
pub mod trimming;

// 重导出以兼容旧的模块名称
pub use data_profile as profile;
pub use missing_handler as fill_missing;

pub use data_profile::*;
pub use missing_handler::*;
pub use winsorize::*;
pub use trimming::*;
