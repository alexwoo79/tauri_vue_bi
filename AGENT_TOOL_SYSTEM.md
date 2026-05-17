# Agent 工具调用系统实现总结

## 概述

本文档记录了 tauri-vue-bi 项目中 Agent 工具调用系统的完整实现。该系统允许 LLM（大语言模型）通过工具调用来执行数据查询、分析和导出等操作。

## 架构设计

### 核心组件

1. **LLM 客户端层** (`src-tauri/src/llm/`)
   - 统一的 `LLMClient` Trait
   - OpenAI 和 Claude 提供商实现
   - 支持工具调用的消息格式

2. **Agent 状态机** (`src-tauri/src/agent/state_machine.rs`)
   - 对话循环逻辑
   - 工具调用解析和执行
   - SSE 流式响应

3. **工具模块** (`src-tauri/src/agent/tools/`)
   - 数据工具（查询、分析、清洗）
   - 导出工具（Excel、PPT、报告、看板）

## 关键数据结构

### ToolCall

```rust
pub struct ToolCall {
    pub id: String,                    // 工具调用 ID
    pub function: ToolCallFunction,    // 函数调用详情
}

pub struct ToolCallFunction {
    pub name: String,      // 函数名称
    pub arguments: String, // JSON 格式的参数
}
```

### Message（扩展）

```rust
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,  // 新增：工具调用列表
    pub tool_call_id: Option<String>,       // 新增：工具响应 ID
}
```

### ChatResponse（扩展）

```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub usage: Option<TokenUsage>,
    pub tool_calls: Option<Vec<ToolCall>>,  // 新增：工具调用
}
```

## 工作流程

### 1. 用户发送消息

```
用户: "帮我分析一下销售数据的趋势"
```

### 2. Agent 构建消息历史

```rust
let mut messages = vec![
    Message::system(system_prompt),
    // ... 历史消息 ...
    Message::user("帮我分析一下销售数据的趋势"),
];
```

### 3. 调用 LLM

```rust
let response = client.chat(messages).await?;
```

### 4. LLM 返回工具调用

LLM 可能返回：
```json
{
  "content": "",
  "tool_calls": [
    {
      "id": "call_123",
      "function": {
        "name": "get_schema",
        "arguments": "{}"
      }
    }
  ]
}
```

### 5. Agent 执行工具

```rust
if let Some(tool_calls) = &response.tool_calls {
    // 发送 ToolStart 事件
    tx.send(SseEvent::ToolStart { ... }).await.ok();
    
    // 执行工具
    let results = Self::execute_tool_calls(tool_calls).await?;
    
    // 添加工具调用到消息历史
    messages.push(Message::assistant_with_tools(...));
    
    // 添加工具响应
    for (tool_call, result) in tool_calls.iter().zip(results.iter()) {
        messages.push(Message::tool(result, tool_call.id.clone()));
    }
    
    // 发送 ToolEnd 事件
    tx.send(SseEvent::ToolEnd { ... }).await.ok();
    
    // 继续下一轮迭代
    continue;
}
```

### 6. 多轮迭代直到完成

Agent 会持续调用 LLM，直到：
- LLM 返回纯文本响应（无工具调用）
- 达到最大迭代次数
- 用户请求停止

## 支持的工具

### 数据工具

| 工具名称 | 功能 | 状态 |
|---------|------|------|
| `get_schema` | 获取数据集结构 | ✅ 已实现 |
| `query_data` | 执行 SQL 查询 | ✅ 已实现 |
| `profile_data` | 数据概况分析 | ✅ 已实现 |
| `clean_data` | 数据清洗 | ⚠️ 部分实现 |
| `run_analysis` | 统计分析 | ⚠️ 占位符 |
| `generate_chart` | 生成图表 | ⚠️ 占位符 |

### 导出工具

| 工具名称 | 功能 | 状态 |
|---------|------|------|
| `propose_excel_export` | 提议 Excel 导出方案 | ✅ 已实现 |
| `propose_ppt_outline` | 提议 PPT 大纲 | ✅ 已实现 |
| `propose_report_outline` | 提议报告大纲 | ✅ 已实现 |
| `propose_dashboard_outline` | 提议看板大纲 | ✅ 已实现 |
| `export_excel` | 导出 Excel 文件 | ⚠️ 占位符 |
| `generate_ppt` | 生成 PPT | ⚠️ 占位符 |
| `export_report` | 生成报告 | ⚠️ 占位符 |
| `generate_dashboard` | 生成看板 | ⚠️ 占位符 |

## SSE 事件类型

Agent 通过以下事件类型向前端推送实时状态：

```rust
pub enum SseEvent {
    TextDelta { content: String },        // 文本增量
    Text { content: String },             // 完整文本
    ToolStart { tool: String, display: String },  // 工具开始
    ToolEnd { tool: String },             // 工具结束
    ChartHtml { html: String },           // 图表 HTML
    ChartRef { chart_id: String },        // 图表引用
    Reasoning { content: String },        // 推理链
    Usage { ... },                        // Token 使用统计
    PptOutline { ... },                   // PPT 大纲
    ExcelOutline { ... },                 // Excel 导出计划
    ReportOutline { ... },                // 报告大纲
    DashboardOutline { ... },             // 看板大纲
    Error { message: String },            // 错误
    Stopped,                              // 已停止
    Done,                                 // 完成
}
```

## 示例对话流程

### 场景：用户请求数据分析

```
User: "帮我看看销售数据的整体情况"

Agent (LLM): 
  → 调用 get_schema() 工具
  
Tool Result:
  DataFrame Schema (1000 rows, 5 columns):
    date  DATE
    product  STRING
    sales  FLOAT
    region  STRING
    quantity  INTEGER

Agent (LLM):
  → 调用 profile_data() 工具
  
Tool Result:
  {
    "rows": 1000,
    "columns": 5,
    "columns_detail": [...]
  }

Agent (LLM):
  → 返回文本总结
  
Final Response:
  "数据集包含 1000 条销售记录，有 5 个字段：日期、产品、销售额、地区、数量..."
```

### 场景：用户请求导出 Excel

```
User: "/export 把销售数据导出为 Excel"

Agent (LLM):
  → 调用 propose_excel_export() 工具
  
Tool Result:
  {
    "tables": ["sales_data"],
    "filename": "export_20260516.xlsx",
    "markdown": "# Excel 导出计划\n..."
  }

Agent (LLM):
  → 返回 PptOutline 事件
  
Frontend:
  → 显示导出预览，等待用户确认

User: (点击确认按钮)

Agent:
  → 接收 ppt_confirm 命令
  → 调用 export_excel() 工具
  → 返回文件路径
```

## 技术要点

### 1. 异步流式处理

使用 `tokio` 和 `futures` 实现完全异步的流式处理：

```rust
pub async fn run_stream(
    &self,
    session_id: &str,
    params: AgentRunParams,
) -> Result<mpsc::Receiver<SseEvent>>
```

### 2. 消息历史管理

维护完整的对话历史，包括工具调用和响应：

```rust
// Assistant 消息带工具调用
messages.push(Message::assistant_with_tools(
    response.content.clone(),
    tool_calls.clone(),
));

// Tool 响应消息
messages.push(Message::tool(
    result.clone(),
    tool_call.id.clone(),
));
```

### 3. 错误处理

使用 `anyhow` 进行统一错误处理：

```rust
let result = match function_name.as_str() {
    "get_schema" => {
        data_tools::tool_get_schema()
            .unwrap_or_else(|e| format!("Error: {}", e))
    }
    // ...
};
```

### 4. 借用检查器友好

在流式处理中避免借用问题：

```rust
// ❌ 错误：借用冲突
let lines: Vec<&str> = buf.split('\n').collect();
buf = lines.last().unwrap_or(&"").to_string();

// ✅ 正确：使用 owned String
let lines: Vec<String> = buf.split('\n').map(|s| s.to_string()).collect();
if let Some(last_line) = lines.last() {
    buf = last_line.clone();
}
```

## 未来改进方向

1. **工具注册系统**
   - 动态工具发现
   - MCP (Model Context Protocol) 集成
   - 工具版本管理

2. **并行工具执行**
   - 多个独立工具同时执行
   - 减少总响应时间

3. **工具缓存**
   - 缓存常用查询结果
   - 减少重复调用

4. **工具权限控制**
   - 敏感操作需要用户确认
   - 基于角色的访问控制

5. **更丰富的错误恢复**
   - 工具调用失败时的重试策略
   - 降级方案

## 测试建议

### 单元测试

```rust
#[tokio::test]
async fn test_execute_tool_calls() {
    let tool_calls = vec![
        ToolCall {
            id: "test_1".to_string(),
            function: ToolCallFunction {
                name: "get_schema".to_string(),
                arguments: "{}".to_string(),
            },
        }
    ];
    
    let results = BusinessAgent::execute_tool_calls(&tool_calls).await.unwrap();
    assert!(!results.is_empty());
}
```

### 集成测试

```rust
#[tokio::test]
async fn test_full_conversation_with_tools() {
    // 创建会话
    let session_id = create_session("openai").await?;
    
    // 发送消息
    let events = chat_stream(
        session_id,
        "分析销售数据".to_string(),
        None, None, None, None, None, None, None, None
    ).await?;
    
    // 验证收到 ToolStart 和 ToolEnd 事件
    // ...
}
```

## 参考资料

- [OpenAI Function Calling](https://platform.openai.com/docs/guides/function-calling)
- [Anthropic Tool Use](https://docs.anthropic.com/claude/docs/tool-use)
- [Model Context Protocol](https://modelcontextprotocol.io/)

---

**作者**: alex  
**日期**: 2026-05-16  
**版本**: 1.0
