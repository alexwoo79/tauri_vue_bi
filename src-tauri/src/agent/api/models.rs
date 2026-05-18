// src-tauri/src/agent/api/models.rs
//
// LLM 模型管理 API - 对标 Python 的 api/models.py
//
// 提供：
// - 获取可用模型列表
// - 获取默认模型配置
// - 设置/清除内置模型配置

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub enable_thinking: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetBuiltinRequest {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub enable_thinking: bool,
}

/// 获取默认模型配置
#[tauri::command]
pub fn get_model_defaults() -> std::collections::HashMap<String, ModelInfo> {
    let mut defaults = std::collections::HashMap::new();

    defaults.insert("openai".to_string(), ModelInfo {
        provider: "openai".to_string(),
        base_url: Some("https://api.openai.com/v1".to_string()),
        model: Some("gpt-4o".to_string()),
        context_window: Some(128000),
        max_output_tokens: Some(4096),
        enable_thinking: false,
    });

    defaults.insert("claude".to_string(), ModelInfo {
        provider: "claude".to_string(),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        model: Some("claude-3-5-sonnet".to_string()),
        context_window: Some(200000),
        max_output_tokens: Some(8192),
        enable_thinking: true,
    });

    defaults
}

/// 获取支持的模型提供商列表
#[tauri::command]
pub fn list_model_providers() -> Vec<String> {
    vec![
        "openai".to_string(),
        "claude".to_string(),
        "qwen".to_string(),
        "deepseek".to_string(),
        "custom".to_string(),
    ]
}