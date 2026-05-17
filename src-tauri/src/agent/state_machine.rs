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

use crate::llm::{ChatChunk, ChatResponse, LLMClient, Message, MessageRole, ToolCall};
use crate::agent::session::{ChatSession, SessionManager};
use crate::agent::tools::{data_tools, export_tools, chart_tools};
use crate::agent::prompts;  // ✅ 新增：导入 prompts 模块

/// SSE 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    TextDelta { content: String },
    Text { content: String },
    ToolStart { tool: String, display: String },
    ToolEnd { tool: String },
    ChartHtml { html: String },
    ChartRef { chart_id: String },
    ChartPlaceholder { index: usize },
    Reasoning { content: String },
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
        session_total_input: u32,
        session_total_output: u32,
    },
    PptOutline {
        title: String,
        slides: Vec<serde_json::Value>,
        markdown: String,
    },
    ExcelOutline {
        tables: Vec<String>,
        filename: String,
        markdown: String,
    },
    ReportOutline {
        title: String,
        sections: Vec<serde_json::Value>,
        markdown: String,
    },
    DashboardOutline {
        name: String,
        widgets: Vec<serde_json::Value>,
        markdown: String,
    },
    PptScheme { scheme: String },
    Error { message: String },
    Stopped,
    Done,
}

impl SseEvent {
    /// 序列化为 SSE 格式字符串
    pub fn to_sse_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        format!("data: {}\n\n", json)
    }
}

/// Agent 运行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunParams {
    pub user_message: String,
    pub command: Option<String>, // 斜杠命令，如 "ppt", "export"
    pub ppt_title: Option<String>,
    pub ppt_slides: Option<Vec<serde_json::Value>>,
    pub excel_tables: Option<Vec<String>>,
    pub excel_filename: Option<String>,
    pub report_title: Option<String>,
    pub report_sections: Option<Vec<serde_json::Value>>,
    pub dashboard_name: Option<String>,
    pub dashboard_widgets: Option<Vec<serde_json::Value>>,
}

/// Agent 状态机
pub struct BusinessAgent {
    client: Arc<dyn LLMClient>,
    model: String,
    enable_thinking: bool,
    max_iterations: usize,
    session_manager: Arc<Mutex<SessionManager>>,
}

impl BusinessAgent {
    const MAX_ITERATIONS_DEFAULT: usize = 100;

    /// 创建新的 Agent 实例
    pub fn new(
        client: Arc<dyn LLMClient>,
        model: String,
        enable_thinking: bool,
        session_manager: Arc<Mutex<SessionManager>>,
    ) -> Self {
        Self {
            client,
            model,
            enable_thinking,
            max_iterations: Self::MAX_ITERATIONS_DEFAULT,
            session_manager,
        }
    }

    /// 运行 Agent 对话循环（异步流式）
    pub async fn run_stream(
        &self,
        session_id: &str,
        params: AgentRunParams,
    ) -> Result<mpsc::Receiver<SseEvent>> {
        let (tx, rx) = mpsc::channel(128);

        // ✅ 修复：使用公共方法获取会话
        let session = {
            let guard = self.session_manager.lock().unwrap();
            guard.get_session(session_id).cloned()
        };

        let Some(mut session) = session else {
            tx.try_send(SseEvent::Error {
                message: "会话不存在".to_string(),
            })
            .ok();
            return Ok(rx);
        };

        // 重置停止标志
        session.cancel_requested = false;

        let client = Arc::clone(&self.client);
        let model = self.model.clone();
        let enable_thinking = self.enable_thinking;
        let max_iterations = self.max_iterations;
        let session_manager = Arc::clone(&self.session_manager);
        let session_id = session_id.to_string();

        tokio::spawn(async move {
            if let Err(e) = Self::run_loop(
                tx,
                session,
                params,
                client,
                model,
                enable_thinking,
                max_iterations,
                session_manager,
                session_id,
            )
            .await
            {
                eprintln!("[Agent] Error in run_loop: {:?}", e);
            }
        });

        Ok(rx)
    }

    /// 核心对话循环
    async fn run_loop(
        tx: mpsc::Sender<SseEvent>,
        mut session: ChatSession,
        params: AgentRunParams,
        client: Arc<dyn LLMClient>,
        model: String,
        enable_thinking: bool,
        max_iterations: usize,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
    ) -> Result<()> {
        // 快速路径：确认命令直接 bypass LLM
        if let Some(cmd) = &params.command {
            match cmd.as_str() {
                "ppt_confirm" => {
                    return Self::handle_ppt_confirm(tx, session, params, session_manager, session_id).await;
                }
                "excel_confirm" => {
                    return Self::handle_excel_confirm(tx, session, params, session_manager, session_id).await;
                }
                "report_confirm" => {
                    return Self::handle_report_confirm(tx, session, params, session_manager, session_id).await;
                }
                "dashboard_confirm" => {
                    return Self::handle_dashboard_confirm(tx, session, params, session_manager, session_id).await;
                }
                _ => {}
            }
        }

        // 构建系统提示
        let system_prompt = Self::build_system_prompt(params.command.as_deref());

        // 准备消息历史
        let mut messages = vec![Message::system(system_prompt)];

        // ✅ 修复：使用 recent_history() 方法获取最近的历史消息
        for msg in session.recent_history(20) {
            let role = match msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::System,
            };
            messages.push(Message {
                role,
                content: msg.content.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // 添加当前用户消息
        messages.push(Message::user(&params.user_message));

        let mut collected_text = String::new();
        let mut all_reasoning = Vec::new();
        let mut force_propose = false;
        let propose_commands = ["ppt", "ppt_revise", "export", "excel_revise", 
                                "report", "report_revise", "dashboard", "dashboard_revise"];

        for iteration in 0..max_iterations {
            // 检查停止请求
                {
                let should_stop = {
                    let guard = session_manager.lock().unwrap();
                    guard.get_sessions().get(&session_id).map(|s| s.cancel_requested).unwrap_or(false)
                };
                if should_stop {
                    tx.send(SseEvent::Stopped).await.ok();
                    tx.send(SseEvent::Done).await.ok();
                    return Ok(());
                }
            }

            // 如果需要强制提议（propose 阶段）
            if force_propose {
                let nudge = Self::build_propose_nudge(params.command.as_deref());
                messages.push(Message::user(nudge));
                force_propose = false;
            }

            // 确定 max_tokens
            let max_tokens = if force_propose || params.command.as_ref().map_or(false, |c| propose_commands.contains(&c.as_str())) {
                16384
            } else {
                8192
            };

            // 调用 LLM
            // ✅ 暂时禁用 Claude thinking mode，统一使用流式
            // let response = if enable_thinking && model.starts_with("claude") {
            //     // Claude thinking mode: 非流式
            //     client.chat(messages.clone()).await?
            // } else {
                // 流式模式
                let tools = crate::agent::tools_schema::get_agent_tools();  // ✅ 获取工具列表
                let stream = client.chat_stream_with_tools(messages.clone(), &tools, None).await?;  // ✅ 传递工具列表和 tool_choice
                let response = Self::process_stream(
                    stream,
                    &tx,
                    &mut collected_text,
                    &mut all_reasoning,
                    &params.command,
                    &propose_commands,
                )
                .await?;

            // 处理工具调用
            if let Some(tool_calls) = &response.tool_calls {
                if !tool_calls.is_empty() {
                    // 发送工具开始事件
                    for tool_call in tool_calls {
                        tx.send(SseEvent::ToolStart {
                            tool: tool_call.function.name.clone(),
                            display: format!("正在调用工具: {}", tool_call.function.name),
                        })
                        .await
                        .ok();
                    }

                    // 执行工具调用
                    let tool_results = Self::execute_tool_calls(tool_calls).await?;

                    // 添加工具调用到消息历史
                    messages.push(Message::assistant_with_tools(
                        response.content.clone(),
                        tool_calls.clone(),
                    ));

                    // 添加工具响应到消息历史
                    for (tool_call, result) in tool_calls.iter().zip(tool_results.iter()) {
                        messages.push(Message::tool(
                            result.clone(),
                            tool_call.id.clone(),
                        ));
                    }

                    // 发送工具结束事件
                    for tool_call in tool_calls {
                        tx.send(SseEvent::ToolEnd {
                            tool: tool_call.function.name.clone(),
                        })
                        .await
                        .ok();
                    }

                    // 继续下一轮迭代
                    continue;
                }
            }

            // 最终文本响应
            if !all_reasoning.is_empty() {
                tx.send(SseEvent::Reasoning {
                    content: all_reasoning.join("\n\n---\n\n"),
                })
                .await
                .ok();
            }

            tx.send(SseEvent::Text {
                content: response.content.clone(),
            })
            .await
            .ok();

            // 保存到会话历史（使用别名方法）
            session.add_user(&params.user_message);
            session.add_assistant(&response.content);

            // 更新 session_manager 中的会话
            {
                let mut guard = session_manager.lock().unwrap();
                let sessions = guard.get_sessions_mut();
                if let Some(sess) = sessions.get_mut(&session_id) {
                    *sess = session;
                }
            }

            tx.send(SseEvent::Done).await.ok();
            return Ok(());
        }

        // 达到最大迭代次数
        tx.send(SseEvent::Text {
            content: "分析完成（已达到最大工具调用次数）。Analysis complete (max iterations reached).".to_string(),
        })
        .await
        .ok();
        tx.send(SseEvent::Done).await.ok();

        Ok(())
    }

    /// 处理流式响应
    async fn process_stream(
        mut stream: Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>,
        tx: &mpsc::Sender<SseEvent>,
        collected_text: &mut String,
        all_reasoning: &mut Vec<String>,
        command: &Option<String>,
        propose_commands: &[&str],
    ) -> Result<ChatResponse> {
        let mut content_parts = Vec::new();
        let mut reasoning_parts = Vec::new();
        let mut usage_data = None;
        let mut final_tool_calls: Option<Vec<ToolCall>> = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            if let Some(content) = chunk.content {
                content_parts.push(content.clone());
                // 如果不是 propose 命令，实时发送文本增量
                if !command.as_ref().map_or(false, |c| propose_commands.contains(&c.as_str())) {
                    tx.send(SseEvent::TextDelta { content }).await.ok();
                }
            }

            if let Some(reasoning) = chunk.reasoning {
                reasoning_parts.push(reasoning);
            }

            // ✅ 修正：检查是否是 tool_calls 结束
            if chunk.finish_reason == Some("tool_calls".to_string()) {
                // 这是最终的完整工具调用列表
                final_tool_calls = chunk.tool_calls;
                break; // 结束流处理
            }

            if chunk.finish_reason == Some("stop".to_string()) {
                break;
            }
        }

        let full_content = content_parts.join("");
        let reasoning_content = if reasoning_parts.is_empty() {
            None
        } else {
            Some(reasoning_parts.join(""))
        };

        if let Some(reasoning) = &reasoning_content {
            all_reasoning.push(reasoning.clone());
        }

        collected_text.push_str(&full_content);

        Ok(ChatResponse {
            content: full_content,
            reasoning: reasoning_content,
            usage: usage_data,
            tool_calls: final_tool_calls,  // ✅ 使用累积的最终结果
        })
    }

    /// 执行工具调用
    async fn execute_tool_calls(tool_calls: &[ToolCall]) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let function_name = &tool_call.function.name;
            let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                .context(format!("Failed to parse arguments for tool: {}", function_name))?;

            let result = match function_name.as_str() {
                // 数据查询工具
                "get_schema" => {
                    data_tools::tool_get_schema()
                        .unwrap_or_else(|e| format!("Error: {}", e))
                }
                "query_data" => {
                    let sql = arguments["sql"].as_str().unwrap_or("");
                    data_tools::tool_query_data(sql)
                        .unwrap_or_else(|e| format!("Error: {}", e))
                }
                "profile_data" => {
                    let table_name = arguments["table_name"].as_str();
                    let columns = arguments["columns"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    });
                    match data_tools::tool_profile_data(table_name, columns) {
                        Ok(json) => serde_json::to_string_pretty(&json).unwrap_or_default(),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                "clean_data" => {
                    let operation = arguments["operation"].as_str().unwrap_or("");
                    let table_name = arguments["table_name"].as_str();
                    let columns = arguments["columns"].as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    });
                    let fill_method = arguments["fill_method"].as_str().unwrap_or("forward");
                    let lower_pct = arguments["lower_pct"].as_f64().unwrap_or(1.0);
                    let upper_pct = arguments["upper_pct"].as_f64().unwrap_or(99.0);
                    
                    data_tools::tool_clean_data(operation, table_name, columns, fill_method, lower_pct, upper_pct)
                        .unwrap_or_else(|e| format!("Error: {}", e))
                }
                
                // 导出工具
                "propose_excel_export" => {
                    let tables = arguments["tables"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    let filename = arguments["filename"].as_str().map(|s| s.to_string());
                    
                    match export_tools::tool_propose_excel_export(tables, filename, None) {
                        Ok(json) => serde_json::to_string_pretty(&json).unwrap_or_default(),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                "propose_ppt_outline" => {
                    let title = arguments["title"].as_str().unwrap_or("").to_string();
                    let slides_json = &arguments["slides"];
                    
                    // TODO: 解析 slides JSON 为 PptSlide 数组
                    match export_tools::tool_propose_ppt_outline(title, vec![]) {
                        Ok(outline) => serde_json::to_string_pretty(&outline).unwrap_or_default(),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                "propose_report_outline" => {
                    let title = arguments["title"].as_str().unwrap_or("").to_string();
                    
                    match export_tools::tool_propose_report_outline(title, vec![]) {
                        Ok(outline) => serde_json::to_string_pretty(&outline).unwrap_or_default(),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                "propose_dashboard_outline" => {
                    let name = arguments["name"].as_str().unwrap_or("").to_string();
                    
                    match export_tools::tool_propose_dashboard_outline(name, vec![]) {
                        Ok(outline) => serde_json::to_string_pretty(&outline).unwrap_or_default(),
                        Err(e) => format!("Error: {}", e),
                    }
                }
                
                // 图表生成工具
                "generate_chart" => {
                    let chart_type = arguments["chart_type"].as_str().unwrap_or("bar");
                    let title = arguments["title"].as_str().unwrap_or("图表").to_string();
                    let color_scheme = arguments["color_scheme"].as_str().unwrap_or("mckinsey").to_string();
                    
                    // 解析字段映射
                    let field_mapping = chart_tools::FieldMapping {
                        x: arguments["x"].as_str().map(|s| s.to_string()),
                        y: arguments["y"].as_str().map(|s| s.to_string()),
                        series: arguments["series"].as_str().map(|s| s.to_string()),
                        label: arguments["label"].as_str().map(|s| s.to_string()),
                        value: arguments["value"].as_str().map(|s| s.to_string()),
                        size: arguments["size"].as_str().map(|s| s.to_string()),
                        color: arguments["color"].as_str().map(|s| s.to_string()),
                        group: arguments["group"].as_str().map(|s| s.to_string()),
                        order: arguments["order"].as_str().map(|s| s.to_string()),
                        dimensions: arguments["dimensions"].as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        }),
                    };
                    
                    let options = chart_tools::ChartOptions {
                        title,
                        color_scheme,
                        sort: arguments["sort"].as_bool().unwrap_or(true),
                        top_n: arguments["top_n"].as_u64().map(|n| n as usize),
                        orientation: arguments["orientation"].as_str().map(|s| s.to_string()),
                    };
                    
                    match chart_tools::tool_generate_chart(chart_type, field_mapping, options) {
                        Ok(chart_result) => {
                            // 返回 ECharts spec 的 JSON 字符串
                            serde_json::to_string_pretty(&chart_result.echarts_spec).unwrap_or_default()
                        }
                        Err(e) => format!("Error: {}", e),
                    }
                }
                
                _ => {
                    format!("Unknown tool: {}", function_name)
                }
            };

            results.push(result);
        }

        Ok(results)
    }

    /// 构建系统提示
    fn build_system_prompt(command: Option<&str>) -> String {
        let base = include_str!("prompts/system_prompt.md");
        
        // ✅ 动态生成图表指南并替换占位符
        let chart_guide = prompts::build_chart_guide();
        let base_with_charts = base.replace("{{CHART_GUIDE}}", &chart_guide);
        
        if let Some(cmd) = command {
            if let Some(hint) = prompts::get_command_hint(cmd) {
                return format!("{}\n\n[ACTIVE COMMAND: /{}]\n{}", base_with_charts, cmd, hint);
            }
        }
        
        base_with_charts
    }

    /// 构建提议提示
    fn build_propose_nudge(command: Option<&str>) -> String {
        match command {
            Some("ppt") | Some("ppt_revise") => {
                "All data has been gathered in the tool results above. \
                 Call propose_ppt_outline with a COMPLETE slides array (8–15 slides). \
                 CRITICAL: use ONLY real numbers, labels, and values extracted from \
                 the tool results in this conversation — do NOT fabricate or invent data."
                    .to_string()
            }
            Some("export") | Some("excel_revise") => {
                "Call propose_excel_export now with the tables list and an optional filename. \
                 Output ONLY the tool call — no surrounding text."
                    .to_string()
            }
            Some("dashboard") | Some("dashboard_revise") => {
                "All data schema information has been gathered. \
                 Call propose_dashboard_outline with a complete widgets array (2-6 widgets)."
                    .to_string()
            }
            _ => {
                "Compose the report outline from the conversation above and call \
                 propose_report_outline with title and sections."
                    .to_string()
            }
        }
    }

    // 快速路径处理函数（TODO: 实现具体逻辑）
    async fn handle_ppt_confirm(
        tx: mpsc::Sender<SseEvent>,
        session: ChatSession,
        params: AgentRunParams,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
    ) -> Result<()> {
        // TODO: 实现 PPT 生成逻辑
        tx.send(SseEvent::Done).await.ok();
        Ok(())
    }

    async fn handle_excel_confirm(
        tx: mpsc::Sender<SseEvent>,
        session: ChatSession,
        params: AgentRunParams,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
    ) -> Result<()> {
        // TODO: 实现 Excel 导出逻辑
        tx.send(SseEvent::Done).await.ok();
        Ok(())
    }

    async fn handle_report_confirm(
        tx: mpsc::Sender<SseEvent>,
        session: ChatSession,
        params: AgentRunParams,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
    ) -> Result<()> {
        // TODO: 实现报告生成逻辑
        tx.send(SseEvent::Done).await.ok();
        Ok(())
    }

    async fn handle_dashboard_confirm(
        tx: mpsc::Sender<SseEvent>,
        session: ChatSession,
        params: AgentRunParams,
        session_manager: Arc<Mutex<SessionManager>>,
        session_id: String,
    ) -> Result<()> {
        // TODO: 实现看板生成逻辑
        tx.send(SseEvent::Done).await.ok();
        Ok(())
    }
}
