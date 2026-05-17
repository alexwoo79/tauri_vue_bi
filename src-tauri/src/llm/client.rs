// src-tauri/src/llm/client.rs
//
// LLM 客户端统一接口
//
// 定义所有 LLM 提供商必须实现的 Trait 和通用数据结构

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::pin::Pin;

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 工具调用参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String, // JSON 字符串
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: ToolCallFunction,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            reasoning_content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// ✅ 新增：支持 reasoning_content 的 assistant_with_tools 变体
    pub fn assistant_with_tools_and_reasoning(
        content: impl Into<String>,
        reasoning_content: Option<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            reasoning_content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    pub usage: Option<TokenUsage>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 流式聊天块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub content: Option<String>,      // 增量内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>, // ✅ thinking 模型的推理内容
    pub finish_reason: Option<String>, // 结束原因（stop/length/tool_calls）
    pub tool_calls: Option<Vec<ToolCall>>, // 工具调用（流式中可能分块返回）
}

/// LLM 客户端 Trait
#[async_trait]
pub trait LLMClient: Send + Sync + Debug {
    /// 同步聊天
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse>;

    /// 流式聊天（不带工具）
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>>;

    /// 流式聊天（带工具）
    async fn chat_stream_with_tools(
        &self,
        messages: Vec<Message>,
        tools: &[serde_json::Value],
        tool_choice: Option<&str>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>> {
        // 默认实现：忽略工具参数，调用基础方法
        // 各提供商应重写此方法以支持工具调用
        let _ = (tools, tool_choice);
        self.chat_stream(messages).await
    }

    /// 获取模型名称
    fn model_name(&self) -> &str;

    /// 获取上下文窗口大小（tokens）
    fn context_window(&self) -> usize {
        4096 // 默认值
    }

    /// 获取最大输出长度（tokens）
    fn max_output_tokens(&self) -> usize {
        2048 // 默认值
    }
}
