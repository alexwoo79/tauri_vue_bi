use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use polars::prelude::*;
use tauri::Emitter;
use uuid::Uuid;
use futures::StreamExt;

// ✅ 新增：导入 Agent 模块（使用别名避免冲突）
use crate::agent::state_machine::{BusinessAgent as AgentStateMachine, AgentRunParams, SseEvent};

// ✅ 使用 include_str! 直接嵌入工具 schema JSON，避免模块依赖
use crate::commands::chart::fetch_chart_data_impl;
use crate::df_util::df_to_payload;
use crate::agent::llm::{LLMClient, Message, MessageRole, OpenAIClient,};
use crate::state::GLOBAL_DF;

// ✅ 新增：全局图表存储（用于存储生成的图表 HTML）
static CHART_STORE: Lazy<Arc<Mutex<HashMap<String, String>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// ✅ 新增：执行工具调用的辅助函数（简化版，不依赖 agent 模块）
async fn execute_tool_call(tool_name: &str, args_str: &str) -> Result<String, String> {
    // 解析 JSON 参数
    let args: serde_json::Value = serde_json::from_str(args_str)
        .map_err(|e| format!("Failed to parse tool arguments: {}", e))?;
    
    tracing::info!(tool_name = %tool_name, args = %args_str, "Executing tool");
    
    match tool_name {
        // ✅ get_schema - 获取数据结构
        "get_schema" => {
            let df_guard = GLOBAL_DF.lock().map_err(|e| e.to_string())?;
            let df = df_guard.as_ref().ok_or("No data loaded")?;
            
            let mut schema_parts = Vec::new();
            for field in df.schema().iter_fields() {
                let dtype_str = match field.dtype() {
                    DataType::Int32 | DataType::Int64 => "INTEGER",
                    DataType::Float32 | DataType::Float64 => "FLOAT",
                    DataType::String => "STRING",
                    DataType::Boolean => "BOOLEAN",
                    DataType::Date => "DATE",
                    DataType::Datetime(_, _) => "DATETIME",
                    _ => "OTHER",
                };
                schema_parts.push(format!("  {}  {}", field.name(), dtype_str));
            }
            
            Ok(format!(
                "DataFrame Schema ({} rows, {} columns):\n{}",
                df.height(),
                df.width(),
                schema_parts.join("\n")
            ))
        }
        
        // ⚠️ query_data - 简化实现（需要完整 SQL 引擎支持）
        "query_data" => {
            let sql = args.get("sql")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'sql' parameter".to_string())?;
            
            // TODO: 实现完整的 SQL 查询功能
            // 当前返回提示信息
            Ok(format!(
                "SQL query received: {}\n\nNote: Full SQL execution requires Polars SQL context integration.",
                sql
            ))
        }
        
        // ⚠️ profile_data - 简化实现
        "profile_data" => {
            let df_guard = GLOBAL_DF.lock().map_err(|e| e.to_string())?;
            let df = df_guard.as_ref().ok_or("No data loaded")?;
            
            // 返回基本统计信息
            Ok(format!(
                "Data Profile:\n- Rows: {}\n- Columns: {}\n- Schema available via get_schema",
                df.height(),
                df.width()
            ))
        }
        
        // ⚠️ clean_data - 简化实现
        "clean_data" => {
            let operation = args.get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'operation' parameter".to_string())?;
            
            Ok(format!(
                "Data cleaning requested: '{}'\n\nNote: Full implementation requires Polars DataFrame operations.",
                operation
            ))
        }
        
        // ⚠️ generate_chart - 前端渲染提示
        "generate_chart" => {
            let chart_type = args.get("chart_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'chart_type' parameter".to_string())?;
            
            let title = args.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Chart");
            
            Ok(format!(
                "Chart generation request:\n- Type: {}\n- Title: {}\n\nNote: Frontend will render chart from queried data.",
                chart_type, title
            ))
        }
        
        // ❌ 未知工具
        _ => {
            Err(format!("Unknown tool: {}", tool_name))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: u64,
    /// ✅ 新增：支持 reasoning_content（用于 thinking 模型）
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
    /// ✅ 新增：支持 tool_call_id（用于工具调用结果）
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// ✅ 新增：支持 tool_calls（用于 assistant 的工具调用）
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<crate::agent::llm::ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatSession {
    id: String,
    title: String,
    model_id: String,
    messages: Vec<ChatMessage>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Debug, Default)]
struct SessionManager {
    sessions: HashMap<String, ChatSession>,
}

impl SessionManager {
    fn create_session(&mut self, model_id: &str) -> String {
        let now = now_secs();
        let id = Uuid::new_v4().to_string();
        self.sessions.insert(
            id.clone(),
            ChatSession {
                id: id.clone(),
                title: "新会话".to_string(),
                model_id: model_id.to_string(),
                messages: Vec::new(),
                created_at: now,
                updated_at: now,
            },
        );
        id
    }

    fn delete_session(&mut self, session_id: &str) -> Result<(), String> {
        self.sessions
            .remove(session_id)
            .map(|_| ())
            .ok_or_else(|| format!("Session '{}' not found", session_id))
    }

    fn clear_history(&mut self, session_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;
        session.messages.clear();
        session.updated_at = now_secs();
        Ok(())
    }

    fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;
        let now = now_secs();
        session.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
            reasoning_content: None, // ✅ 普通消息不包含 reasoning_content
            tool_call_id: None,
            tool_calls: None,
        });
        session.updated_at = now;
        if session.title == "新会话" && role == "user" {
            session.title = content.chars().take(20).collect();
        }
        Ok(())
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

static SESSION_MANAGER: Lazy<Arc<Mutex<SessionManager>>> =
    Lazy::new(|| Arc::new(Mutex::new(SessionManager::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResult {
    pub html: String,
    pub chart_type: String,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}

const PLOTLY_JS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../node_modules/plotly.js-dist-min/plotly.min.js"));

fn value_to_js(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => "null".to_string(),
    }
}

fn build_schema_preview(df: &DataFrame) -> String {
    let mut lines = Vec::new();
    lines.push(format!("DataFrame Schema ({} rows, {} columns):", df.height(), df.width()));
    for field in df.schema().iter_fields() {
        let dtype_str = match field.dtype() {
            DataType::Int32 | DataType::Int64 => "INTEGER".to_string(),
            DataType::Float32 | DataType::Float64 => "FLOAT".to_string(),
            DataType::String => "STRING".to_string(),
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Datetime(_, _) => "DATETIME".to_string(),
            other => format!("{other:?}"),
        };
        lines.push(format!("  {}  {}", field.name(), dtype_str));
    }
    lines.join("\n")
}

fn rowmaps_to_hashmaps(rows: &[serde_json::Map<String, serde_json::Value>]) -> Vec<HashMap<String, serde_json::Value>> {
    rows.iter()
        .map(|row| row.clone().into_iter().collect())
        .collect()
}

fn extract_series(
    data: &[HashMap<String, serde_json::Value>],
    field: &str,
) -> Vec<serde_json::Value> {
    data.iter()
        .map(|row| row.get(field).cloned().unwrap_or(serde_json::Value::Null))
        .collect()
}

fn field_has_value(data: &[HashMap<String, serde_json::Value>], field: &str) -> bool {
    data.iter().any(|row| match row.get(field) {
        Some(serde_json::Value::Null) | None => false,
        Some(serde_json::Value::String(s)) => !s.trim().is_empty(),
        Some(_) => true,
    })
}

fn render_plotly_html(
    chart_type: &str,
    title: &str,
    data: &[HashMap<String, serde_json::Value>],
    x_field: &str,
    y_field: &str,
) -> String {
    let x_values = extract_series(data, x_field);
    let y_values = extract_series(data, y_field);

    let x_js = format!(
        "[{}]",
        x_values.iter().map(value_to_js).collect::<Vec<_>>().join(",")
    );
    let y_js = format!(
        "[{}]",
        y_values.iter().map(value_to_js).collect::<Vec<_>>().join(",")
    );
    let safe_title = serde_json::to_string(title).unwrap_or_else(|_| "\"Chart\"".to_string());

    let trace_js = match chart_type {
        "Line_Chart" => format!(
            "{{ x: {x_js}, y: {y_js}, type: 'scatter', mode: 'lines+markers', name: {title} }}",
            x_js = x_js,
            y_js = y_js,
            title = safe_title
        ),
        "Pie_Chart" => format!(
            "{{ labels: {x_js}, values: {y_js}, type: 'pie', textinfo: 'label+percent' }}",
            x_js = x_js,
            y_js = y_js
        ),
        _ => format!(
            "{{ x: {x_js}, y: {y_js}, type: 'bar', name: {title} }}",
            x_js = x_js,
            y_js = y_js,
            title = safe_title
        ),
    };

    format!(
        r#"<!doctype html>
<html>
<head>
    <meta charset="utf-8" />
  <style>
    html, body, #chart {{ width: 100%; height: 100%; margin: 0; padding: 0; background: #fff; }}
        #chart-error {{ display: none; padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: #b42318; background: #fff7f7; border: 1px solid #fecdca; border-radius: 8px; margin: 12px; white-space: pre-wrap; }}
  </style>
</head>
<body>
        <div id="chart-error"></div>
        <div id="chart"></div>
        <script>
{plotly_js}
        </script>
  <script>
        (function() {{
            const trace = {trace};
            const layout = {{
                title: {title},
                margin: {{ l: 52, r: 20, t: 52, b: 52 }},
                paper_bgcolor: '#ffffff',
                plot_bgcolor: '#ffffff'
            }};
            const errorBox = document.getElementById('chart-error');
            try {{
                if (!window.Plotly) {{
                    throw new Error('Plotly 脚本加载失败');
                }}
                Plotly.newPlot('chart', [trace], layout, {{ responsive: true }});
            }} catch (err) {{
                const message = err instanceof Error ? err.message : String(err);
                if (errorBox) {{
                    errorBox.style.display = 'block';
                    errorBox.textContent = '图表渲染失败：' + message;
                }}
            }}
        }})();
  </script>
</body>
</html>"#,
                plotly_js = PLOTLY_JS,
        trace = trace_js,
        title = safe_title,
    )
}

fn build_system_prompt(command: Option<&str>) -> String {
    let base = "You are a professional data analysis assistant. Respond in concise Chinese unless the user requests otherwise.";
    
    // ✅ 添加数据源信息到系统提示词
    let data_info = match GLOBAL_DF.lock() {
        Ok(guard) => {
            if let Some(df) = guard.as_ref() {
                format!("\n\nAvailable data:\n{}", build_schema_preview(df))
            } else {
                "\n\nNo data loaded yet.".to_string()
            }
        }
        Err(_) => "\n\nData source status unknown.".to_string(),
    };
    
    match command {
        Some("chart") => format!(
            "{}\n\n[ACTIVE COMMAND: /chart]\nThe user issued /chart. Your primary goal is to generate one or more data visualizations. Query the relevant data first, then call generate_chart. End with a brief interpretation of the chart.\n{}",
            base, data_info
        ),
        Some("sql") => format!(
            "{}\n\n[ACTIVE COMMAND: /sql]\nThe user issued /sql. Execute the SQL they described and show the results clearly formatted as a table, then provide a short insight.\n{}",
            base, data_info
        ),
        Some("ppt") => format!(
            "{}\n\n[ACTIVE COMMAND: /ppt]\nThe user issued /ppt. Call propose_ppt_outline first and do not generate a PPT directly. If data is needed, gather it first, then build a complete outline from real results only.\n{}",
            base, data_info
        ),
        Some("report") => format!(
            "{}\n\n[ACTIVE COMMAND: /report]\nThe user issued /report. If charts are requested, gather the relevant data first and generate the chart from the results. Then call propose_report_outline and output nothing after the tool call.\n{}",
            base, data_info
        ),
        Some("dashboard") => format!(
            "{}\n\n[ACTIVE COMMAND: /dashboard]\nThe user issued /dashboard. First gather schema and key metrics, then call propose_dashboard_outline with 2-6 widgets. Use only real table and column names from the schema.\n{}",
            base, data_info
        ),
        Some("export") => format!(
            "{}\n\n[ACTIVE COMMAND: /export]\nThe user issued /export. Call propose_excel_export and do not export directly this turn.\n{}",
            base, data_info
        ),
        Some(other) => format!("{}\n\n[ACTIVE COMMAND: /{}]\n{}", base, other, data_info),
        None => format!("{}\n{}", base, data_info),
    }
}

fn build_llm_client(
    provider: &str,
    model: Option<&str>,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> Result<Box<dyn LLMClient>, String> {
    let key = api_key
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "缺少 API Key，请在模型配置中填写后重试。".to_string())?;

    let provider_lc = provider.to_lowercase();
    // ✅ 暂时禁用 Claude，统一使用 OpenAI
    let model_name = model.unwrap_or("gpt-4o-mini");

    // if provider_lc == "claude" {
    //     return Ok(Box::new(ClaudeClient::new(key, model_name.to_string())));
    // }

    let mut client = OpenAIClient::new(key, model_name.to_string());
    if let Some(url) = base_url {
        if !url.trim().is_empty() {
            client = client.with_base_url(url.to_string());
        }
    }
    Ok(Box::new(client))
}

#[tauri::command]
pub async fn create_session(model_id: String) -> Result<String, String> {
    let mut manager = SESSION_MANAGER.lock().map_err(|e| e.to_string())?;
    Ok(manager.create_session(&model_id))
}

#[tauri::command]
pub async fn delete_session(session_id: String) -> Result<(), String> {
    let mut manager = SESSION_MANAGER.lock().map_err(|e| e.to_string())?;
    manager.delete_session(&session_id)
}

#[tauri::command]
pub async fn list_sessions() -> Result<Vec<SessionInfo>, String> {
    let manager = SESSION_MANAGER.lock().map_err(|e| e.to_string())?;
    let mut out: Vec<SessionInfo> = manager
        .sessions
        .values()
        .map(|s| SessionInfo {
            id: s.id.clone(),
            title: s.title.clone(),
            created_at: s.created_at,
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

#[tauri::command]
pub async fn clear_session_history(session_id: String) -> Result<(), String> {
    let mut manager = SESSION_MANAGER.lock().map_err(|e| e.to_string())?;
    manager.clear_history(&session_id)
}

#[tauri::command]
pub async fn stop_session(_session_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn generate_chart(
    chart_type: String,
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: HashMap<String, serde_json::Value>,
    options: Option<HashMap<String, serde_json::Value>>,
) -> Result<ChartResult, String> {
    if data.is_empty() {
        return Err("图表数据为空，请先加载数据集后再生成图表。".to_string());
    }

    let title = options
        .as_ref()
        .and_then(|o| o.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Chart");

    let x_field = mapping
        .get("x")
        .and_then(|v| v.as_str())
        .unwrap_or("x");
    let y_field = mapping
        .get("y")
        .and_then(|v| v.as_str())
        .unwrap_or("y");

    if !field_has_value(&data, x_field) {
        return Err(format!("图表字段 '{}' 在当前数据中没有可用值。", x_field));
    }

    if !field_has_value(&data, y_field) {
        return Err(format!("图表字段 '{}' 在当前数据中没有可用值。", y_field));
    }

    let html = render_plotly_html(&chart_type, title, &data, x_field, y_field);

    Ok(ChartResult {
        html,
        chart_type,
        warnings: vec![],
        meta: serde_json::json!({
            "rows": data.len(),
            "x": x_field,
            "y": y_field,
        }),
    })
}

#[tauri::command]
pub async fn list_chart_types() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({"chart_id":"Bar_Chart","name":"柱状图","category":"基础"}),
        serde_json::json!({"chart_id":"Line_Chart","name":"折线图","category":"基础"}),
        serde_json::json!({"chart_id":"Pie_Chart","name":"饼图","category":"基础"}),
    ])
}

#[tauri::command]
pub async fn chart_workflow(
    app: tauri::AppHandle,
    chart_type: String,
    x_col: String,
    y_col: String,
    title: Option<String>,
) -> Result<(), String> {
    let df = {
        let guard = GLOBAL_DF.lock().map_err(|e| e.to_string())?;
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "当前没有可用于制图的数据，请先加载或切换数据集。".to_string())?
    };

    let chart_title = title.unwrap_or_else(|| format!("{} 图表", chart_type));

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "thinking",
            "content": "正在按照工作流准备图表..."
        }),
    )
    .map_err(|e| e.to_string())?;

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_start",
            "tool": "get_schema",
            "display": "读取数据结构..."
        }),
    )
    .map_err(|e| e.to_string())?;

    let schema_text = build_schema_preview(&df);
    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_result",
            "tool": "get_schema",
            "content": schema_text
        }),
    )
    .ok();

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_start",
            "tool": "query_data",
            "display": format!("执行查询: SELECT {}, {} ...", x_col, y_col)
        }),
    )
    .map_err(|e| e.to_string())?;

    let result_df = fetch_chart_data_impl(
        &df,
        &x_col,
        &[y_col.clone()],
        None,
        "none",
        true,
        200,
    )
    .map_err(|e| e.to_string())?;

    let payload = df_to_payload(&result_df, Some(200)).map_err(|e| e.to_string())?;
    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_result",
            "tool": "query_data",
            "content": format!("查询返回 {} 行。", payload.total_rows)
        }),
    )
    .ok();

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_start",
            "tool": "generate_chart",
            "display": format!("生成 {} 图表...", chart_type)
        }),
    )
    .map_err(|e| e.to_string())?;

    let rows = rowmaps_to_hashmaps(&payload.rows);
    let html = render_plotly_html(&chart_type, &chart_title, &rows, &x_col, &y_col);

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "chart_html",
            "html": html,
            "chart_type": chart_type,
            "meta": {
                "xCol": x_col,
                "yCol": y_col,
                "rows": payload.total_rows
            }
        }),
    )
    .map_err(|e| e.to_string())?;

    app.emit(
        "sse-event",
        serde_json::json!({
            "type": "tool_result",
            "tool": "generate_chart",
            "content": "图表生成完成"
        }),
    )
    .ok();

    app.emit("sse-event", serde_json::json!({ "type": "done" }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn chat_stream(
    app: tauri::AppHandle,
    session_id: String,
    user_message: String,
    command: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<(), String> {
    let provider = provider.unwrap_or_else(|| "openai".to_string());
    let model = model.unwrap_or_else(|| "gpt-4".to_string());

    tracing::info!(
        session_id = %session_id,
        command = ?command,
        model = %model,
        "Starting chat stream with AgentStateMachine"
    );

    // ✅ 构建 LLM 客户端
    let llm_client = build_llm_client(
        &provider,
        Some(&model),
        api_key.as_deref(),
        base_url.as_deref(),
    )?;

    // ✅ 创建兼容的 session manager（使用 agent::session::SessionManager）
    let compat_session_mgr = {
        let local_mgr = SESSION_MANAGER.lock().map_err(|e| e.to_string())?;
        let mut agent_mgr = crate::agent::session::SessionManager::new();
        
        // 迁移所有会话数据
        for (id, local_session) in &local_mgr.sessions {
            // 创建新的 ChatSession
            let mut new_session = crate::agent::session::ChatSession::new(Some(local_session.model_id.clone()));
            new_session.id = id.clone();
            new_session.title = local_session.title.clone();
            new_session.created_at = local_session.created_at.to_string();
            new_session.updated_at = local_session.updated_at.to_string();
            
            // 迁移消息
            for msg in &local_session.messages {
                let role = match msg.role.as_str() {
                    "user" => crate::agent::session::MessageRole::User,
                    "assistant" => crate::agent::session::MessageRole::Assistant,
                    "system" => crate::agent::session::MessageRole::System,
                    _ => crate::agent::session::MessageRole::User,
                };
                
                new_session.messages.push(crate::agent::session::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    role,
                    content: msg.content.clone(),
                    reasoning_content: msg.reasoning_content.clone(),
                    tool_calls: msg.tool_calls.clone(),
                    timestamp: Some(msg.timestamp.to_string()),
                });
            }
            
            agent_mgr.get_sessions_mut().insert(id.clone(), new_session);
        }
        
        Arc::new(Mutex::new(agent_mgr))
    };

    // ✅ 创建 BusinessAgent（将 Box 转换为 Arc）
    let agent = AgentStateMachine::new(
        Arc::from(llm_client),  // ✅ 转换 Box<dyn LLMClient> 为 Arc<dyn LLMClient>
        model.clone(),
        false, // enable_thinking - 暂时禁用
        compat_session_mgr,
    );

    // ✅ 构建 AgentRunParams
    let params = AgentRunParams {
        user_message: user_message.clone(),
        command,
        ppt_title: None,
        ppt_slides: None,
        excel_tables: None,
        excel_filename: None,
        report_title: None,
        report_sections: None,
        dashboard_name: None,
        dashboard_widgets: None,
    };

    // ✅ 调用 run_stream 获取 SSE 事件流
    let mut rx = agent.run_stream(&session_id, params).await
        .map_err(|e| format!("Agent run failed: {}", e))?;

    // ✅ 转发 SSE 事件到 Tauri emit
    while let Some(event) = rx.recv().await {
        match event {
            SseEvent::TextDelta { content } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "text_delta",
                    "content": content
                })).ok();
            }
            SseEvent::Text { content } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "text",
                    "content": content
                })).ok();
            }
            SseEvent::ToolStart { tool, display } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "tool_start",
                    "tool": tool,
                    "display": display
                })).ok();
            }
            SseEvent::ToolEnd { tool } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "tool_end",
                    "tool": tool
                })).ok();
            }
            SseEvent::ToolResult { tool, content } => {
                // ✅ 关键修复：转发工具结果到前端，让LLM能看到工具返回的数据
                tracing::info!(
                    tool = %tool,
                    content_len = content.len(),
                    "ToolResult event forwarded to frontend"
                );
                app.emit("sse-event", serde_json::json!({
                    "type": "tool_result",
                    "tool": tool,
                    "content": content
                })).ok();
            }
            SseEvent::ChartHtml { html } => {
                // ✅ 生成 chart_id 并存储到全局 CHART_STORE
                let chart_id = Uuid::new_v4().to_string();
                {
                    let mut store = CHART_STORE.lock().map_err(|e| e.to_string())?;
                    store.insert(chart_id.clone(), html.clone());
                }
                
                tracing::info!(chart_id = %chart_id, "Chart stored");
                
                // ✅ 发送 chart_ref 事件给前端
                app.emit("sse-event", serde_json::json!({
                    "type": "chart_ref",
                    "chart_id": chart_id
                })).ok();
            }
            SseEvent::ChartRef { chart_id } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "chart_ref",
                    "chart_id": chart_id
                })).ok();
            }
            SseEvent::ChartPlaceholder { index } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "chart_placeholder",
                    "index": index
                })).ok();
            }
            SseEvent::Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
                context_window,
                max_output_tokens,
                session_total_input,
                session_total_output,
            } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "usage",
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": total_tokens,
                    "context_window": context_window,
                    "max_output_tokens": max_output_tokens,
                    "session_total_input": session_total_input,
                    "session_total_output": session_total_output
                })).ok();
            }
            SseEvent::PptOutline { title, slides, markdown } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "ppt_outline",
                    "title": title,
                    "slides": slides,
                    "markdown": markdown
                })).ok();
            }
            SseEvent::ExcelOutline { tables, filename, markdown } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "excel_outline",
                    "tables": tables,
                    "filename": filename,
                    "markdown": markdown
                })).ok();
            }
            SseEvent::ReportOutline { title, sections, markdown } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "report_outline",
                    "title": title,
                    "sections": sections,
                    "markdown": markdown
                })).ok();
            }
            SseEvent::DashboardOutline { name, widgets, markdown } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "dashboard_outline",
                    "name": name,
                    "widgets": widgets,
                    "markdown": markdown
                })).ok();
            }
            SseEvent::PptScheme { scheme } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "ppt_scheme",
                    "scheme": scheme
                })).ok();
            }
            SseEvent::Error { message } => {
                app.emit("sse-event", serde_json::json!({
                    "type": "error",
                    "message": message
                })).ok();
            }
            SseEvent::Stopped => {
                app.emit("sse-event", serde_json::json!({
                    "type": "stopped"
                })).ok();
            }
            SseEvent::Done => {
                tracing::info!("Agent stream completed");
                app.emit("sse-event", serde_json::json!({
                    "type": "done"
                })).ok();
                break; // ✅ 退出循环
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn export_excel(_session_id: String, _config: serde_json::Value) -> Result<String, String> {
    Err("export_excel not yet implemented".to_string())
}

#[tauri::command]
pub async fn generate_ppt(_session_id: String, _config: serde_json::Value) -> Result<String, String> {
    Err("generate_ppt not yet implemented".to_string())
}

#[tauri::command]
pub async fn generate_report(
    _session_id: String,
    _config: serde_json::Value,
) -> Result<String, String> {
    Err("generate_report not yet implemented".to_string())
}

#[tauri::command]
pub async fn generate_dashboard(
    _session_id: String,
    _config: serde_json::Value,
) -> Result<String, String> {
    Err("generate_dashboard not yet implemented".to_string())
}

// ✅ 新增：本地定义工具 Schema（避免依赖 agent 模块）
fn get_agent_tools_local() -> Vec<serde_json::Value> {
    use serde_json::json;
    
    vec![
        // get_schema
        json!({
            "type": "function",
            "function": {
                "name": "get_schema",
                "description": "Get the full schema of the connected data source — tables, columns, types, and row counts. Always call this first when the user asks about data you haven't seen yet.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        }),
        
        // query_data
        json!({
            "type": "function",
            "function": {
                "name": "query_data",
                "description": "Execute a SQL SELECT query and return the results as a table.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sql": {
                            "type": "string",
                            "description": "A valid SQL SELECT statement using actual column/table names from the schema."
                        }
                    },
                    "required": ["sql"]
                }
            }
        }),
        
        // profile_data
        json!({
            "type": "function",
            "function": {
                "name": "profile_data",
                "description": "Profile the data to show statistics like count, mean, std, min, max, null counts, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Table name to profile (leave empty for first table)"
                        }
                    },
                    "required": []
                }
            }
        }),
        
        // clean_data
        json!({
            "type": "function",
            "function": {
                "name": "clean_data",
                "description": "Clean the data by handling missing values, capping extremes, or trimming rows.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "description": "Operation type: 'fill_na', 'winsorize', or 'trimming'"
                        }
                    },
                    "required": ["operation"]
                }
            }
        }),
        
        // generate_chart
        json!({
            "type": "function",
            "function": {
                "name": "generate_chart",
                "description": "Generate a chart from queried data. Call query_data first, then use this tool.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "chart_type": {
                            "type": "string",
                            "description": "Chart type: bar, line, pie, scatter, area, heatmap, boxplot, stacked_bar, grouped_bar, histogram, waterfall"
                        },
                        "title": {
                            "type": "string",
                            "description": "Chart title"
                        }
                    },
                    "required": ["chart_type"]
                }
            }
        }),
    ]
}
