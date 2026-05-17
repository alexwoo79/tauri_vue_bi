// src-tauri/src/commands/mod.rs
//
// 命令模块统一导出（Command Module Re-exports）

pub mod agent_chat;  // 新增 Rust Agent 命令
pub mod chart;
pub mod clean;
pub mod dataset;
pub mod datasource;
pub mod gantt;
pub mod groupby;
pub mod llm_test;  // 新增 LLM 测试命令
pub mod loader;
pub mod melt;
pub mod merge;
pub mod pivot;
pub mod python_agent;
pub mod save;
pub mod time_analysis;
