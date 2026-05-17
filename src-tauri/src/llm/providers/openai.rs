use crate::llm::client::*;
use async_trait::async_trait;
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tracing::{debug, info};

#[derive(Debug)]
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    context_window: usize,
    max_output_tokens: usize,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: Option<usize>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Serialize)]
struct ToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefinition,
}

#[derive(Serialize)]
struct FunctionDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiToolFunction,
}

#[derive(Serialize, Deserialize)]
struct ApiToolFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatApiResponse {
    choices: Vec<Choice>,
    usage: Option<ApiTokenUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ApiMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ApiTokenUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// 流式响应的数据结构
#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
    role: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<StreamToolCall>>,
}

// ✅ 新增：流式工具调用结构
#[derive(Deserialize, Debug, Clone)]
struct StreamToolCall {
    index: usize,
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Deserialize, Debug, Clone)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

// ✅ 新增：工具调用构建器，用于累积流式参数
#[derive(Debug, Clone)]
struct ToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallBuilder {
    fn new() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            arguments: String::new(),
        }
    }

    fn build(self) -> crate::llm::client::ToolCall {
        crate::llm::client::ToolCall {
            id: self.id,
            function: crate::llm::client::ToolCallFunction {
                name: self.name,
                arguments: self.arguments,
            },
        }
    }
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        let (context_window, max_output_tokens) = match model.as_str() {
            "gpt-4o" => (128000, 16384),
            "gpt-4o-mini" => (128000, 16384),
            "gpt-4-turbo" => (128000, 4096),
            _ => (4096, 2048),
        };

        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
            context_window,
            max_output_tokens,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse> {
        info!(model = %self.model, "Calling OpenAI API");
        debug!(message_count = messages.len(), "Messages prepared");

        let api_messages: Vec<ApiMessage> = messages
            .into_iter()
            .map(|msg| ApiMessage {
                role: format!("{:?}", msg.role).to_lowercase(),
                content: msg.content,
                tool_calls: msg.tool_calls.map(|tc| {
                    tc.iter().map(|t| ApiToolCall {
                        id: t.id.clone(),
                        call_type: "function".to_string(),
                        function: ApiToolFunction {
                            name: t.function.name.clone(),
                            arguments: t.function.arguments.clone(),
                        },
                    }).collect()
                }),
                tool_call_id: msg.tool_call_id,
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: Some(self.max_output_tokens),
            stream: false,
            tools: None,
            tool_choice: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        let api_response: ChatApiResponse = response.json().await?;

        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let usage = api_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        info!(
            tokens = usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
            "OpenAI response received"
        );

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            reasoning: None, // OpenAI 暂不支持推理链
            usage,
            tool_calls: None, // TODO: 解析工具调用
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>> {
        // ✅ 调用不带工具的版本（保持向后兼容）
        self.chat_stream_with_tools(messages, &[], None).await
    }

    async fn chat_stream_with_tools(
        &self,
        messages: Vec<Message>,
        tools: &[serde_json::Value],
        tool_choice: Option<&str>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>> {
        // ✅ 委托给具体实现
        OpenAIClient::chat_stream_with_tools(self, messages, tools, tool_choice).await
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

// ✅ 新增：实现带工具调用的 chat_stream_with_tools
impl OpenAIClient {
    pub async fn chat_stream_with_tools(
        &self,
        messages: Vec<Message>,
        tools: &[serde_json::Value],
        tool_choice: Option<&str>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<ChatChunk>> + Send>>> {
        info!(model = %self.model, "Calling OpenAI API with tools (streaming)");
        debug!(message_count = messages.len(), tool_count = tools.len(), "Messages and tools prepared");

        let api_messages: Vec<ApiMessage> = messages
            .into_iter()
            .map(|msg| ApiMessage {
                role: format!("{:?}", msg.role).to_lowercase(),
                content: msg.content,
                tool_calls: msg.tool_calls.map(|tc| {
                    tc.iter().map(|t| ApiToolCall {
                        id: t.id.clone(),
                        call_type: "function".to_string(),
                        function: ApiToolFunction {
                            name: t.function.name.clone(),
                            arguments: t.function.arguments.clone(),
                        },
                    }).collect()
                }),
                tool_call_id: msg.tool_call_id,
            })
            .collect();

        // 将 JSON Value 转换为 ToolDefinition
        let api_tools: Vec<ToolDefinition> = tools
            .iter()
            .filter_map(|tool_json| {
                // 期望格式: {"type": "function", "function": {...}}
                let func = tool_json.get("function")?;
                let name = func.get("name")?.as_str()?.to_string();
                let description = func.get("description")?.as_str()?.to_string();
                let parameters = func.get("parameters")?.clone();
                
                Some(ToolDefinition {
                    tool_type: "function".to_string(),
                    function: FunctionDefinition {
                        name,
                        description,
                        parameters,
                    },
                })
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: Some(self.max_output_tokens),
            stream: true,
            tools: if api_tools.is_empty() { None } else { Some(api_tools) },
            tool_choice: tool_choice.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        // 使用与 chat_stream 相同的流处理逻辑
        let stream = try_stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buf = String::new();
            
            let mut tc_accumulator: std::collections::HashMap<usize, ToolCallBuilder> = std::collections::HashMap::new();

            while let Some(chunk_result) = bytes_stream.next().await {
                let chunk = chunk_result?;
                let text = String::from_utf8_lossy(&chunk);
                buf.push_str(&text);

                let lines: Vec<String> = buf.split('\n').map(|s| s.to_string()).collect();
                
                if let Some(last_line) = lines.last() {
                    buf = last_line.clone();
                } else {
                    buf.clear();
                }
                
                for line in &lines[..lines.len().saturating_sub(1)] {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with(":") {
                        continue;
                    }

                    if line.starts_with("data: ") {
                        let json_data = &line[6..];
                        
                        if json_data == "[DONE]" {
                            break;
                        }

                        if let Ok(stream_resp) = serde_json::from_str::<StreamResponse>(json_data) {
                            if let Some(choice) = stream_resp.choices.first() {
                                if let Some(delta_tool_calls) = &choice.delta.tool_calls {
                                    for dtc in delta_tool_calls {
                                        let builder = tc_accumulator.entry(dtc.index).or_insert_with(|| ToolCallBuilder::new());
                                        
                                        if let Some(id) = &dtc.id {
                                            builder.id = id.clone();
                                        }
                                        
                                        if let Some(func) = &dtc.function {
                                            if let Some(name) = &func.name {
                                                builder.name.push_str(name);
                                            }
                                            if let Some(args) = &func.arguments {
                                                builder.arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                                
                                if choice.finish_reason == Some("tool_calls".to_string()) {
                                    let final_tool_calls: Vec<crate::llm::client::ToolCall> = tc_accumulator
                                        .values()
                                        .map(|b| b.clone().build())
                                        .collect();
                                    
                                    yield ChatChunk {
                                        content: None,
                                        reasoning: None,
                                        finish_reason: Some("tool_calls".to_string()),
                                        tool_calls: Some(final_tool_calls),
                                    };
                                    return;
                                }
                                
                                let chunk = ChatChunk {
                                    content: choice.delta.content.clone(),
                                    reasoning: None,
                                    finish_reason: choice.finish_reason.clone(),
                                    tool_calls: None,
                                };
                                yield chunk;
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore] // 需要设置 OPENAI_API_KEY 环境变量
    async fn test_openai_chat() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("Please set OPENAI_API_KEY environment variable");

        let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string());

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
    async fn test_openai_chat_stream() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("Please set OPENAI_API_KEY environment variable");

        let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string());

        let messages = vec![Message::user("Tell me a short story.")];

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

    #[tokio::test]
    #[ignore]
    async fn test_openai_with_custom_url() {
        // 测试自定义 baseURL（如 Azure OpenAI、本地代理等）
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string())
            .with_base_url("https://api.openai.com/v1".to_string());

        let messages = vec![Message::user("Say hi")];
        let response = client.chat(messages).await.unwrap();

        assert!(!response.content.is_empty());
    }
}
