pub mod client;
pub mod providers;
pub mod config;

pub use client::*;
pub use config::LLMConfigManager;
// pub use providers::{OpenAIClient, ClaudeClient};
pub use providers::{OpenAIClient};  // ✅ 暂时禁用 Claude，专注于调试 OpenAI
