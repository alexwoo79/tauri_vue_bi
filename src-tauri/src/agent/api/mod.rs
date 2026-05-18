// src-tauri/src/agent/api/mod.rs
//
// Agent API 模块 - 对标 Python Data-Analysis-Agent 的 api/
//
// 规范 Agent 的独立 API 模块，包含：
// - models.rs    : LLM 模型管理 API
// - chat.rs      : 对话 API（会话管理、聊天流式响应）
// - datasource.rs: 数据源管理 API
// - dashboard.rs : 看板 API
// - output.rs    : 输出/下载 API
// - system.rs    : 系统 API
// - color_schemes.rs: 配色方案 API

pub mod models;
pub mod chat;
pub mod datasource;
pub mod dashboard;
pub mod output;
pub mod system;
pub mod color_schemes;