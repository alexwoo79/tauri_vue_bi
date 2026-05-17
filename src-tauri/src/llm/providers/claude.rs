use crate::llm::client::*;
use async_trait::async_trait;
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, info};

#[derive(Debug)]
pub struct ClaudeClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    context_window: usize,
    max_output_tokens: usize,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    max_tokens: usize,
    stream: bool,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    usage: Option<ClaudeTokenUsage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize, Debug)]  // ✅ 添加 Debug
struct ClaudeTokenUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// 流式响应的数据结构
#[derive(Deserialize, Debug)]
struct StreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<StreamDelta>,
    message: Option<StreamMessage>,
    content_block: Option<StreamContentBlock>, // ✅ 修正：使用新的结构名
    index: Option<usize>,
}

// ✅ 新增：流式内容块（支持工具调用）
#[derive(Deserialize, Debug)]
struct StreamContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
    name: Option<String>, // 工具名称
    input: Option<serde_json::Value>, // 工具参数
}

#[derive(Deserialize, Debug)]
struct StreamDelta {
    text: Option<String>,
    #[serde(rename = "type")]
    delta_type: Option<String>,
    partial_json: Option<String>, // ✅ 新增：工具参数的增量 JSON
}

#[derive(Deserialize, Debug)]  // ✅ 添加 Debug
struct StreamMessage {
    usage: Option<ClaudeTokenUsage>,
}

// ✅ 新增：累积工具调用的构建器
#[derive(Debug, Clone)]  // ✅ 添加 Clone
struct ClaudeToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

impl ClaudeToolCallBuilder {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: String::new(),
            arguments: String::new(),
        }
    }

    fn build(self) -> crate::llm::client::ToolCall {
        crate::llm::client::ToolCall {
            id: self.id,
            function: crate::llm::client::ToolCallFunction {  // ✅ 修正：使用 ToolCallFunction
                name: self.name,
                arguments: self.arguments,
            },
        }
    }
}

impl ClaudeClient {
    pub fn new(api_key: String, model: String) -> Self {
        let (context_window, max_output_tokens) = match model.as_str() {
            "claude-3-5-sonnet-20241022" => (200000, 8192),
            "claude-3-5-haiku-20241022" => (200000, 8192),
            "claude-3-opus-20240229" => (200000, 4096),
            _ => (200000, 4096),
        };

        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
            context_window,
            max_output_tokens,
        }
    }
}

#[async_trait]
impl LLMClient for ClaudeClient {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse> {
        info!(model = %self.model, "Calling Claude API");
        debug!(message_count = messages.len(), "Messages prepared");

        // Claude 要求第一条消息必须是 user
        let mut claude_messages: Vec<ClaudeMessage> = Vec::new();
        let mut system_prompt: Option<String> = None;

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    system_prompt = Some(msg.content);
                }
                MessageRole::User => {
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: msg.content,
                    });
                }
                MessageRole::Assistant => {
                    claude_messages.push(ClaudeMessage {
                        role: "assistant".to_string(),
                        content: msg.content,
                    });
                }
                MessageRole::Tool => {
                    // Claude 的工具消息处理方式不同，这里简化处理
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: format!("[Tool Result] {}", msg.content),
                    });
                }
            }
        }

        let request = ClaudeRequest {
            model: self.model.clone(),
            messages: claude_messages,
            max_tokens: self.max_output_tokens,
            stream: false,
        };

        let mut req_builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request);

        // 如果有 system prompt，添加到 header（Claude 的特殊处理）
        if let Some(_system) = &system_prompt {
            req_builder = req_builder.header("anthropic-beta", "prompt-caching-2024-07-31");
            // 注意：Claude 的 system prompt 通过 header 传递有特殊限制
            // 生产环境需要更复杂的逻辑来处理长 system prompt
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {}", error_text);
        }

        let api_response: ClaudeResponse = response.json().await?;

        let content = api_response
            .content
            .first()
            .map(|block| block.text.clone())
            .unwrap_or_default();

        let usage = api_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        info!(
            tokens = usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
            "Claude response received"
        );

        Ok(ChatResponse {
            content,
            reasoning: None,
            usage,
            tool_calls: None, // TODO: 解析工具调用
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>> {
        info!(model = %self.model, "Calling Claude API (streaming)");
        debug!(message_count = messages.len(), "Messages prepared");

        // 准备消息（与同步方法相同）
        let mut claude_messages: Vec<ClaudeMessage> = Vec::new();
        let mut system_prompt: Option<String> = None;

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    system_prompt = Some(msg.content);
                }
                MessageRole::User => {
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: msg.content,
                    });
                }
                MessageRole::Assistant => {
                    claude_messages.push(ClaudeMessage {
                        role: "assistant".to_string(),
                        content: msg.content,
                    });
                }
                MessageRole::Tool => {
                    claude_messages.push(ClaudeMessage {
                        role: "user".to_string(),
                        content: format!("[Tool Result] {}", msg.content),
                    });
                }
            }
        }

        let request = ClaudeRequest {
            model: self.model.clone(),
            messages: claude_messages,
            max_tokens: self.max_output_tokens,
            stream: true,
        };

        let mut req_builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("accept", "text/event-stream")
            .json(&request);

        if let Some(_system) = &system_prompt {
            req_builder = req_builder.header("anthropic-beta", "prompt-caching-2024-07-31");
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {}", error_text);
        }

        // 使用 async-stream 创建流
        let stream = try_stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buf = String::new();
            
            // ✅ 新增：按 index 累积工具调用
            let mut tc_accumulator: std::collections::HashMap<usize, ClaudeToolCallBuilder> = std::collections::HashMap::new();

            while let Some(chunk_result) = bytes_stream.next().await {
                let chunk = chunk_result?;
                let text = String::from_utf8_lossy(&chunk);
                buf.push_str(&text);

                // 按行处理 SSE 数据
                let lines: Vec<String> = buf.split('\n').map(|s| s.to_string()).collect();
                
                // 保留最后一个可能不完整的行
                if let Some(last_line) = lines.last() {
                    buf = last_line.clone();
                } else {
                    buf.clear();
                }
                
                // 处理所有完整的行（除了最后一行）
                for line in &lines[..lines.len().saturating_sub(1)] {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with(":") {
                        continue;
                    }

                    if line.starts_with("data: ") {
                        let json_data = &line[6..];
                        
                        // 解析事件
                        if let Ok(event) = serde_json::from_str::<StreamEvent>(json_data) {
                            match event.event_type.as_str() {
                                "content_block_start" => {
                                    // ✅ 工具调用开始
                                    if let Some(block) = event.content_block {
                                        if block.block_type == Some("tool_use".to_string()) {
                                            if let Some(index) = event.index {
                                                let id = format!("tool_{}", index);
                                                let mut builder = ClaudeToolCallBuilder::new(&id);
                                                // ✅ 保存工具名称
                                                if let Some(name) = block.name {
                                                    builder.name = name;
                                                }
                                                tc_accumulator.insert(index, builder);
                                            }
                                        }
                                    }
                                }
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        // ✅ 累积工具参数
                                        if let Some(partial_json) = delta.partial_json {
                                            if let Some(index) = event.index {
                                                if let Some(builder) = tc_accumulator.get_mut(&index) {
                                                    builder.arguments.push_str(&partial_json);
                                                }
                                            }
                                        }
                                        
                                        // 发送文本增量
                                        if let Some(text) = delta.text {
                                            yield ChatChunk {
                                                content: Some(text),
                                                reasoning: None,
                                                finish_reason: None,
                                                tool_calls: None,
                                            };
                                        }
                                    }
                                }
                                "content_block_stop" => {
                                    // ✅ 工具块结束（无需额外处理，名称和参数已累积）
                                }
                                "message_delta" => {
                                    // 消息结束，可能有 usage 信息
                                    if let Some(_msg) = event.message {
                                        // Claude 在 message_delta 中发送 usage
                                    }
                                }
                                "message_stop" => {
                                    // ✅ 流结束，检查是否有工具调用
                                    let has_tools = !tc_accumulator.is_empty();
                                    
                                    if has_tools {
                                        // ✅ 发送最终的完整工具调用列表
                                        let final_tool_calls: Vec<crate::llm::client::ToolCall> = tc_accumulator
                                            .into_values()
                                            .map(|b| b.build())
                                            .collect();
                                        
                                        yield ChatChunk {
                                            content: None,
                                            reasoning: None,
                                            finish_reason: Some("tool_calls".to_string()),
                                            tool_calls: Some(final_tool_calls),
                                        };
                                    } else {
                                        // 普通文本结束
                                        yield ChatChunk {
                                            content: None,
                                            reasoning: None,
                                            finish_reason: Some("stop".to_string()),
                                            tool_calls: None,
                                        };
                                    }
                                    break;
                                }

                                _ => {}
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn context_window(&self) -> usize {
        self.context_window
    }

    fn max_output_tokens(&self) -> usize {
        self.max_output_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore] // 需要设置 ANTHROPIC_API_KEY 环境变量
    async fn test_claude_chat() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("Please set ANTHROPIC_API_KEY environment variable");

        let client = ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string());

        let messages = vec![Message::user("Hello! Who are you?")];

        let response = client.chat(messages).await.unwrap();

        assert!(!response.content.is_empty());
        println!("Response: {}", response.content);

        if let Some(usage) = &response.usage {
            println!("Tokens used: {}", usage.total_tokens);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_claude_chat_stream() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("Please set ANTHROPIC_API_KEY environment variable");

        let client = ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string());

        let messages = vec![Message::user("Tell me a short joke.")];

        let mut stream = client.chat_stream(messages).await.unwrap();

        let mut full_content = String::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(content) = &chunk.content {
                        print!("{}", content);
                        full_content.push_str(content);
                    }
                    if chunk.finish_reason.is_some() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Stream error: {}", e);
                    break;
                }
            }
        }
        println!("\n\nFull response length: {}", full_content.len());
        assert!(!full_content.is_empty());
    }
}
