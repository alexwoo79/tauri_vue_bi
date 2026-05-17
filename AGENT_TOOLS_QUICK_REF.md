# Agent 工具调用快速参考

## 添加新工具的步骤

### 1. 在工具模块中实现函数

```rust
// src-tauri/src/agent/tools/data_tools.rs 或 export_tools.rs

pub fn tool_your_new_tool(param1: &str, param2: Option<String>) -> Result<String> {
    // 实现工具逻辑
    Ok(format!("Result: {}", param1))
}
```

### 2. 在 state_machine.rs 中注册工具

```rust
// src-tauri/src/agent/state_machine.rs - execute_tool_calls() 函数

let result = match function_name.as_str() {
    // ... 现有工具 ...
    
    "your_new_tool" => {
        let param1 = arguments["param1"].as_str().unwrap_or("");
        let param2 = arguments["param2"].as_str().map(|s| s.to_string());
        
        data_tools::tool_your_new_tool(param1, param2)
            .unwrap_or_else(|e| format!("Error: {}", e))
    }
    
    _ => {
        format!("Unknown tool: {}", function_name)
    }
};
```

### 3. （可选）添加工具定义到 LLM

如果需要 LLM 自动发现工具，需要在调用 LLM 时提供工具定义（JSON Schema 格式）。

---

## 常用代码片段

### 创建带工具调用的消息

```rust
use crate::llm::{Message, ToolCall, ToolCallFunction};

let tool_call = ToolCall {
    id: "call_123".to_string(),
    function: ToolCallFunction {
        name: "get_schema".to_string(),
        arguments: "{}".to_string(),
    },
};

let msg = Message::assistant_with_tools(
    "Let me check the schema.",
    vec![tool_call],
);
```

### 创建工具响应消息

```rust
let tool_response = Message::tool(
    "Schema: date (DATE), sales (FLOAT)",
    "call_123",
);
```

### 发送 SSE 事件

```rust
// 工具开始
tx.send(SseEvent::ToolStart {
    tool: "get_schema".to_string(),
    display: "正在获取数据结构...".to_string(),
}).await.ok();

// 工具结束
tx.send(SseEvent::ToolEnd {
    tool: "get_schema".to_string(),
}).await.ok();

// 文本增量
tx.send(SseEvent::TextDelta {
    content: "Hello".to_string(),
}).await.ok();

// 完成
tx.send(SseEvent::Done).await.ok();
```

---

## 调试技巧

### 1. 查看工具调用日志

在 `execute_tool_calls()` 中添加日志：

```rust
tracing::info!("Executing tool: {}, args: {}", function_name, arguments);
```

### 2. 测试单个工具

```rust
#[tokio::test]
async fn test_get_schema() {
    let result = data_tools::tool_get_schema();
    println!("{:?}", result);
    assert!(result.is_ok());
}
```

### 3. 模拟完整对话

```rust
#[tokio::test]
async fn test_agent_with_tools() {
    // 创建测试会话
    let session_manager = Arc::new(SessionManager::new());
    let session_id = session_manager.create("openai".to_string());
    
    // 创建 Mock LLM 客户端
    let mock_client = Arc::new(MockLLMClient::new());
    
    // 创建 Agent
    let agent = BusinessAgent::new(mock_client, "gpt-4".to_string(), false, session_manager);
    
    // 运行对话
    let params = AgentRunParams {
        user_message: "分析数据".to_string(),
        command: None,
        // ... 其他字段设为 None
    };
    
    let mut rx = agent.run_stream(&session_id, params).await.unwrap();
    
    // 收集事件
    while let Some(event) = rx.recv().await {
        println!("{:?}", event);
        if matches!(event, SseEvent::Done | SseEvent::Error { .. }) {
            break;
        }
    }
}
```

---

## 常见问题

### Q1: 工具调用后 LLM 没有继续对话？

**A**: 检查是否正确将工具响应添加到消息历史：

```rust
// ✅ 正确
messages.push(Message::assistant_with_tools(content, tool_calls.clone()));
for (tool_call, result) in tool_calls.iter().zip(results.iter()) {
    messages.push(Message::tool(result.clone(), tool_call.id.clone()));
}

// ❌ 错误：忘记添加工具响应
messages.push(Message::assistant(content));
```

### Q2: 工具执行报错 "Unknown tool"？

**A**: 检查 `execute_tool_calls()` 中的 match 语句是否包含该工具名称。

### Q3: 流式响应不工作？

**A**: 确保：
1. reqwest 依赖包含 `stream` 特性
2. 使用 `response.bytes_stream()` 而不是 `response.text()`
3. 正确处理 SSE 格式（`data: ` 前缀）

### Q4: 借用检查器错误？

**A**: 避免在迭代时修改被借用的变量：

```rust
// ❌ 错误
let lines: Vec<&str> = buf.split('\n').collect();
buf = lines.last().unwrap_or(&"").to_string(); // buf 仍被 lines 借用

// ✅ 正确
let lines: Vec<String> = buf.split('\n').map(|s| s.to_string()).collect();
if let Some(last) = lines.last() {
    buf = last.clone();
}
```

---

## 性能优化建议

### 1. 并行执行独立工具

```rust
// 如果多个工具调用互不依赖，可以并行执行
let results = futures::future::join_all(
    tool_calls.iter().map(|tc| execute_single_tool(tc))
).await;
```

### 2. 缓存查询结果

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

static QUERY_CACHE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

async fn cached_query(sql: &str) -> Result<String> {
    // 检查缓存
    {
        let cache = QUERY_CACHE.read().await;
        if let Some(result) = cache.get(sql) {
            return Ok(result.clone());
        }
    }
    
    // 执行查询
    let result = tool_query_data(sql)?;
    
    // 写入缓存
    {
        let mut cache = QUERY_CACHE.write().await;
        cache.insert(sql.to_string(), result.clone());
    }
    
    Ok(result)
}
```

### 3. 限制消息历史长度

```rust
// 只保留最近 N 条消息
let recent_messages = session.recent_history(20);
```

---

## 工具调用示例

### 示例 1: 数据查询

```rust
// LLM 决定调用 query_data 工具
{
  "name": "query_data",
  "arguments": {
    "sql": "SELECT region, SUM(sales) as total FROM data GROUP BY region"
  }
}

// Agent 执行工具
let result = tool_query_data("SELECT region, SUM(sales) as total FROM data GROUP BY region");

// 返回结果给 LLM
region	total
East	150000
West	120000
North	180000
South	95000
```

### 示例 2: PPT 大纲生成

```rust
// LLM 调用 propose_ppt_outline
{
  "name": "propose_ppt_outline",
  "arguments": {
    "title": "销售分析报告",
    "slides": [
      {
        "slide_type": "cover",
        "title": "2026年Q1销售分析",
        "content": null
      },
      {
        "slide_type": "content",
        "title": "各地区销售对比",
        "content": "东部地区表现最佳..."
      }
    ]
  }
}

// Agent 返回 PptOutline 事件
tx.send(SseEvent::PptOutline {
    title: "销售分析报告".to_string(),
    slides: [...],
    markdown: "# 销售分析报告\n\n## Slide 1: ...\n...".to_string(),
}).await.ok();
```

---

## API 参考

### SseEvent 枚举

```rust
pub enum SseEvent {
    TextDelta { content: String },
    Text { content: String },
    ToolStart { tool: String, display: String },
    ToolEnd { tool: String },
    ChartHtml { html: String },
    ChartRef { chart_id: String },
    ChartPlaceholder { index: usize },
    Reasoning { content: String },
    Usage { /* ... */ },
    PptOutline { /* ... */ },
    ExcelOutline { /* ... */ },
    ReportOutline { /* ... */ },
    DashboardOutline { /* ... */ },
    PptScheme { scheme: String },
    Error { message: String },
    Stopped,
    Done,
}
```

### AgentRunParams 结构

```rust
pub struct AgentRunParams {
    pub user_message: String,
    pub command: Option<String>,              // 斜杠命令
    pub ppt_title: Option<String>,
    pub ppt_slides: Option<Vec<serde_json::Value>>,
    pub excel_tables: Option<Vec<String>>,
    pub excel_filename: Option<String>,
    pub report_title: Option<String>,
    pub report_sections: Option<Vec<serde_json::Value>>,
    pub dashboard_name: Option<String>,
    pub dashboard_widgets: Option<Vec<serde_json::Value>>,
}
```

---

**最后更新**: 2026-05-16  
**维护者**: alex
