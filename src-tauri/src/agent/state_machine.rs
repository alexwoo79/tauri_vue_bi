// src-tauri/src/agent/state_machine.rs
//
// Agent 状态机 - 实现对话循环和工具调用逻辑
//
// 核心流程：
// 1. 接收用户消息
// 2. 构建系统提示和历史上下文
// 3. 调用 LLM（流式或非流式）
// 4. 解析工具调用或文本响应
// 5. 执行工具并返回结果
// 6. 重复直到完成或达到最大迭代次数

use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::agent::llm::{ChatChunk, ChatResponse, LLMClient, Message, MessageRole, ToolCall};
use crate::agent::session::{ChatSession, SessionManager};
use crate::agent::functions::charts::{generate_chart, recommend_charts};
use crate::agent::functions::charts::base::{FieldMapping, ChartOptions, ChartResult};
use crate::agent::dataops::{data_tools, export_tools};
use crate::agent::prompts;  // ✅ 新增：导入 prompts 模块

/// 常量定义
const MAX_HISTORY_MESSAGES: usize = 20;
const MAX_TOKENS_PROPOSE: u32 = 16384;
const MAX_TOKENS_NORMAL: u32 = 8192;
const PROPOSE_COMMANDS: &[&str] = &["ppt", "ppt_revise", "export", "excel_revise", "report"];

/// Agent 状态
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Ready,
    Thinking,
    Generating,
    ExecutingTool,
    Completed,
    Error,
}

/// Agent 响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResponse {
    Text(String),
    Chart(String),
    ToolResult(String),
    Error(String),
    Done,
}

/// Agent 配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_iterations: usize,
    pub enable_streaming: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".to_string(),
            api_key: String::new(),
            base_url: None,
            max_iterations: 10,
            enable_streaming: true,
        }
    }
}

/// Agent 实例
pub struct Agent {
    config: AgentConfig,
    session: ChatSession,
    llm_client: Box<dyn LLMClient>,
}

impl Agent {
    pub fn new(config: AgentConfig, llm_client: Box<dyn LLMClient>) -> Self {
        Self {
            config,
            session: ChatSession::new(None),
            llm_client,
        }
    }

    /// 运行 Agent 对话循环
    pub async fn run(
        &mut self,
        user_message: &str,
        sender: Option<mpsc::Sender<AgentResponse>>,
    ) -> Result<AgentResponse> {
        // 添加用户消息到会话
        self.session.add_user_message(user_message);

        for iteration in 0..self.config.max_iterations {
            // 构建系统提示
            let system_prompt = prompts::get_system_prompt();
            
            // 获取历史消息
            let mut messages = self.session.get_messages_for_llm();
            
            // 插入系统提示
            messages.insert(0, Message::system(system_prompt));
            
            // 限制历史消息数量
            if messages.len() > MAX_HISTORY_MESSAGES {
                messages = messages[(messages.len() - MAX_HISTORY_MESSAGES)..].to_vec();
            }

            // 调用 LLM
            let response = if self.config.enable_streaming {
                self.call_llm_streaming(&messages, sender.clone()).await?
            } else {
                self.call_llm(&messages).await?
            };

            // 处理响应
            match response {
                AgentResponse::Text(content) => {
                    self.session.add_assistant_message(&content);
                    
                    // 检查是否包含工具调用请求
                    if self.needs_tool_call(&content) {
                        continue;
                    }
                    
                    return Ok(AgentResponse::Text(content));
                }
                AgentResponse::Chart(chart_id) => {
                    self.session.add_assistant_message(&format!("Chart generated: {}", chart_id));
                    return Ok(AgentResponse::Chart(chart_id));
                }
                AgentResponse::ToolResult(result) => {
                    self.session.add_assistant_message(&result);
                    continue;
                }
                AgentResponse::Error(e) => {
                    return Ok(AgentResponse::Error(e));
                }
                AgentResponse::Done => {
                    return Ok(AgentResponse::Done);
                }
            }
        }

        Ok(AgentResponse::Error("Max iterations reached".to_string()))
    }

    /// 调用 LLM（非流式）
    async fn call_llm(&self, messages: &[Message]) -> Result<AgentResponse> {
        let response = self.llm_client.chat(messages.to_vec()).await?;
        
        if let Some(tool_calls) = response.tool_calls {
            return self.execute_tools(tool_calls).await;
        }
        
        Ok(AgentResponse::Text(response.content))
    }

    /// 调用 LLM（流式）
    async fn call_llm_streaming(
        &self,
        messages: &[Message],
        sender: Option<mpsc::Sender<AgentResponse>>,
    ) -> Result<AgentResponse> {
        let tools = self.get_tool_schemas();
        let mut stream = self.llm_client.chat_stream_with_tools(messages.to_vec(), &tools, None).await?;
        
        let mut full_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(content) = chunk.content {
                        full_content.push_str(&content);
                        
                        if let Some(sender) = &sender {
                            sender.send(AgentResponse::Text(content)).await?;
                        }
                    }
                    
                    if let Some(calls) = chunk.tool_calls {
                        tool_calls.extend(calls);
                    }
                    
                    if chunk.finish_reason == Some("tool_calls".to_string()) {
                        return self.execute_tools(tool_calls).await;
                    }
                    
                    if chunk.finish_reason == Some("stop".to_string()) {
                        return Ok(AgentResponse::Text(full_content));
                    }
                }
                Err(e) => {
                    return Ok(AgentResponse::Error(e.to_string()));
                }
            }
        }

        Ok(AgentResponse::Text(full_content))
    }

    /// 执行工具调用
    async fn execute_tools(&self, tool_calls: Vec<ToolCall>) -> Result<AgentResponse> {
        let mut results = Vec::new();
        
        for tool_call in tool_calls {
            let result = self.execute_tool(&tool_call).await;
            results.push(result);
        }
        
        let combined_result = results.join("\n\n");
        Ok(AgentResponse::ToolResult(combined_result))
    }

    /// 执行单个工具
    async fn execute_tool(&self, tool_call: &ToolCall) -> String {
        let tool_name = &tool_call.function.name;
        let arguments = &tool_call.function.arguments;
        
        match tool_name.as_str() {
            "generate_chart" => {
                // 解析图表参数
                match serde_json::from_str::<serde_json::Value>(arguments) {
                    Ok(args) => {
                        let chart_type = args.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar_chart");
                        let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
                        
                        // 获取全局数据
                        let df = crate::state::GLOBAL_DF.lock().unwrap();
                        if let Some(df) = df.as_ref() {
                            // 调用图表生成函数
                            let mapping = FieldMapping::default();
                            let options = ChartOptions {
                                title: Some(title.to_string()),
                                ..ChartOptions::default()
                            };
                            
                            match generate_chart(Some(df), chart_type, Some(mapping), Some(options), None) {
                                Ok(result) => format!("Chart generated: {:?}", result),
                                Err(e) => format!("Chart generation failed: {}", e),
                            }
                        } else {
                            "No data loaded".to_string()
                        }
                    }
                    Err(e) => format!("Failed to parse arguments: {}", e),
                }
            }
            "export_report" => {
                // 调用导出报告工具
                export_tools::tool_export_report("data", vec![]).unwrap_or_else(|e| format!("Export failed: {}", e))
            }
            "get_schema" => {
                // 获取数据 schema
                data_tools::tool_get_schema().unwrap_or_else(|e| format!("Get schema failed: {}", e))
            }
            _ => {
                format!("Unknown tool: {}", tool_name)
            }
        }
    }

    /// 获取工具 Schema 列表
    fn get_tool_schemas(&self) -> Vec<serde_json::Value> {
        crate::agent::tools_schema::get_agent_tools()
    }

    /// 检查是否需要工具调用
    fn needs_tool_call(&self, content: &str) -> bool {
        PROPOSE_COMMANDS.iter().any(|cmd| content.to_lowercase().contains(cmd))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 兼容旧版本的类型定义
// ─────────────────────────────────────────────────────────────────────────────

/// SSE 事件类型（用于流式响应）
#[derive(Debug, Clone)]
pub enum SseEvent {
    TextDelta { content: String },
    Text { content: String },
    ToolStart { tool: String, display: String },
    ToolEnd { tool: String },
    ToolResult { tool: String, content: String },
    ChartHtml { html: String },
    ChartRef { chart_id: String },
    ChartPlaceholder { index: usize },
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        context_window: Option<u32>,
        max_output_tokens: Option<u32>,
        session_total_input: Option<u32>,
        session_total_output: Option<u32>,
    },
    PptOutline { title: String, slides: Vec<String>, markdown: String },
    ExcelOutline { tables: Vec<String>, filename: String, markdown: String },
    ReportOutline { title: String, sections: Vec<String>, markdown: String },
    DashboardOutline { name: String, widgets: Vec<String>, markdown: String },
    PptScheme { scheme: String },
    Error { message: String },
    Stopped,
    Done,
}

/// Agent 运行参数
#[derive(Debug, Clone)]
pub struct AgentRunParams {
    pub user_message: String,
    pub command: Option<String>,
    pub ppt_title: Option<String>,
    pub ppt_slides: Option<Vec<String>>,
    pub excel_tables: Option<Vec<String>>,
    pub excel_filename: Option<String>,
    pub report_title: Option<String>,
    pub report_sections: Option<Vec<String>>,
    pub dashboard_name: Option<String>,
    pub dashboard_widgets: Option<Vec<String>>,
}

/// 兼容旧版本的 BusinessAgent
pub struct BusinessAgent {
    llm_client: Arc<dyn LLMClient>,
    model: String,
    enable_thinking: bool,
    session_manager: Arc<Mutex<crate::agent::session::SessionManager>>,
}

impl BusinessAgent {
    pub fn new(
        llm_client: Arc<dyn LLMClient>,
        model: String,
        enable_thinking: bool,
        session_manager: Arc<Mutex<crate::agent::session::SessionManager>>,
    ) -> Self {
        Self {
            llm_client,
            model,
            enable_thinking,
            session_manager,
        }
    }

    pub async fn run_stream(
        &self,
        session_id: &str,
        params: AgentRunParams,
    ) -> Result<mpsc::Receiver<SseEvent>> {
        let (tx, rx) = mpsc::channel(100);
        
        // 简化实现：发送一个简单的响应
        let _ = tx.send(SseEvent::Text {
            content: format!("Agent received message: {}", params.user_message),
        }).await;
        let _ = tx.send(SseEvent::Done).await;
        
        Ok(rx)
    }
}