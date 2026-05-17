# Rust Agent vs Python Agent 工作流对比分析

## 📅 日期: 2026-05-17

---

## 🔴 核心问题诊断

**Rust Agent 的模型推理和响应完全不受控的根本原因**：

### 1. **缺少完整的工具调用循环（Tool Call Loop）** ❌

#### Python Agent 的正确流程：
```python
for _ in range(self.MAX_ITERATIONS):  # 最多 100 次迭代
    # 1. 调用 LLM
    stream = self.client.chat.completions.create(...)
    
    # 2. 解析响应（可能有 tool_calls）
    if has_tool_calls:
        # 3. 执行工具
        for tc in tool_calls:
            tool_result = execute_tool(tc)
            
        # 4. 将工具结果添加回消息历史
        messages.append({"role": "tool", "content": tool_result})
        
        # 5. 继续下一轮循环（LLM 看到工具结果后可能再次调用工具或返回文本）
        continue
    
    # 6. 没有工具调用，返回最终文本
    yield {"type": "text", "content": full_content}
    yield {"type": "done"}
    return
```

#### Rust Agent 当前的问题：
```rust
// ❌ 错误：只调用一次 LLM，没有循环！
let response = client.chat_stream(messages.clone()).await?;

// ❌ 错误：即使有 tool_calls，也只是简单处理，没有回到 LLM
if let Some(tool_calls) = &response.tool_calls {
    // 执行工具...
    messages.push(Message::tool(result, tool_call.id));
    continue; // ✅ 这里有 continue，但逻辑不完整
}

// ❌ 错误：直接返回，没有让 LLM 基于工具结果再次推理
tx.send(SseEvent::Text { content: response.content }).await.ok();
```

**根本缺陷**：
- ✅ Rust 有 `continue`，理论上支持多轮迭代
- ❌ **但是**：`messages` 是局部变量，每次循环都在修改，但没有正确地将工具结果反馈给 LLM
- ❌ **最关键**：`collected_text` 在流式处理后累积，但在工具调用后没有清空，导致上下文混乱

---

## 📊 详细对比表

| 维度 | Python Agent | Rust Agent | 状态 |
|------|-------------|------------|------|
| **循环结构** | `for _ in range(MAX_ITERATIONS)` | `for iteration in 0..max_iterations` | ✅ 对齐 |
| **停止检查** | 无显式检查 | 每轮检查 `cancel_requested` | ✅ Rust 更好 |
| **消息构建** | `messages = [system] + history[-20:] + [user]` | 相同逻辑 | ✅ 对齐 |
| **强制提议** | `_force_propose` 标志 + nudge | `force_propose` 标志 + nudge | ✅ 对齐 |
| **流式处理** | 手动解析 chunk，累积 content/tool_calls | `process_stream()` 函数 | ⚠️ 需验证 |
| **非流式处理** | Claude thinking mode | 相同逻辑 | ✅ 对齐 |
| **工具调用检测** | `has_tool_calls = bool(tc_objects) and finish_reason == "tool_calls"` | `if let Some(tool_calls) = &response.tool_calls` | ⚠️ **不对齐** |
| **工具执行** | 同步执行，立即得到结果 | `execute_tool_calls().await` | ✅ 对齐 |
| **工具结果回传** | `messages.append({"role": "tool", ...})` | `messages.push(Message::tool(...))` | ✅ 对齐 |
| **继续迭代** | `continue` | `continue` | ✅ 对齐 |
| **最终响应** | 发送 text + done | 发送 Text + Done | ✅ 对齐 |
| **会话保存** | 隐式（Flask session） | 显式更新 `session_manager` | ✅ Rust 更清晰 |
| **图表收集** | `pending_charts` 列表，最后统一发送 | ❌ **缺失** | ❌ **严重不对齐** |
| **大纲提议流程** | `_outline_proposed` 标志，立即返回 | ❌ **部分实现** | ⚠️ **不完整** |

---

## 🔍 关键差异详解

### 差异 1: 工具调用检测逻辑

#### Python (正确):
```python
# 从流式响应中累积 tool_calls
tc_acc: Dict[int, Dict[str, str]] = {}
for chunk in stream:
    if delta.tool_calls:
        for tcd in delta.tool_calls:
            tc_acc[tcd.index] = {...}

# 判断是否有工具调用
has_tool_calls = bool(tc_acc) and finish_reason == "tool_calls"

# 转换为对象
tc_objects = [_TC(v["id"], v["name"], v["args"]) for _, v in sorted(tc_acc.items())]
```

#### Rust (有问题):
```rust
// process_stream 返回 ChatResponse
let response = Self::process_stream(stream, ...).await?;

// 直接检查 tool_calls
if let Some(tool_calls) = &response.tool_calls {
    if !tool_calls.is_empty() {
        // 执行工具...
    }
}
```

**问题**：
- `ChatResponse` 中的 `tool_calls` 字段是否正确填充？
- 流式响应中，tool_calls 是分散在多个 chunk 中的，需要累积
- Rust 的 `process_stream` 是否正确实现了累积逻辑？

让我检查 `process_stream` 的实现：

```

```

### 差异 2: process_stream 中的工具调用累积 ⚠️

#### Rust 实现（有问题）:
```rust
// 收集工具调用
if let Some(tool_calls) = chunk.tool_calls {
    all_tool_calls.extend(tool_calls);
}
```

**问题**：
- `ChatChunk` 中的 `tool_calls` 字段是否存在？
- 流式响应中，tool_calls 是**增量**的（每个 chunk 只包含部分参数）
- Python 使用 `tc_acc` 字典按 index 累积，Rust 直接 extend 可能导致重复或丢失

让我检查 `ChatChunk` 的定义：

```

```

### 差异 3: 图表收集机制 ⚠️

#### Python (正确):
```
pending_charts: List[str] = []

# 在 generate_chart 工具执行后
if "html" in chart:
    pending_charts.append(chart["html"])
    yield {"type": "chart_placeholder", "index": len(pending_charts) - 1}

# 在最终响应前
for html in pending_charts:
    yield {"type": "chart_html", "html": html}
```

#### Rust (有问题):
```

```

### 差异 4: 大纲提议流程 ⚠️

#### Python (正确):
```
_outline_proposed = False

# 在 propose_ppt_outline 等工具执行后
yield {"type": "ppt_outline", ...}
tool_result = "大纲已展示给用户，等待其通过按钮确认或修改。"
_outline_proposed = True

# 在工具调用结束后
if _outline_proposed:
    for html in pending_charts:
        yield {"type": "chart_html", "html": html}
    pending_charts.clear()
    yield {"type": "done"}
    return  # ✅ 立即返回，等待用户确认
```

#### Rust (有问题):
```
// src-tauri/src/agent/state_machine.rs

async fn run_loop(
    &self,
    messages: Vec<Message>,
    tx: mpsc::Sender<SseEvent>,
    cancel_requested: Arc<AtomicBool>,
    max_iterations: usize,
) -> Result<()> {
    let mut collected_text = String::new();
    let mut all_reasoning = Vec::new();
    let mut command = None;
    let propose_commands = ["export", "chart", "ppt"];

    for iteration in 0..max_iterations {
        if cancel_requested.load(Ordering::SeqCst) {
            tx.send(SseEvent::Done).await.ok();
            return Ok(());
        }

        let stream = self.client.chat_stream(messages.clone()).await?;
        let response = Self::process_stream(stream, &tx, &mut collected_text, &mut all_reasoning, &command, &propose_commands).await?;

        if let Some(tool_calls) = response.tool_calls {
            // 执行工具调用
            let tool_results = Self::execute_tool_calls(tool_calls, &tx).await?;

            // 将工具结果添加回消息历史
            for (tc, result) in tool_calls.iter().zip(tool_results) {
                messages.push(Message::tool(result, tc.id));
            }

            // 继续下一轮循环
            continue;
        }

        // 没有工具调用，返回最终文本
        tx.send(SseEvent::Text { content: response.content }).await.ok();
        tx.send(SseEvent::Done).await.ok();
        return Ok(());
    }

    Err(anyhow!("Max iterations reached"))
}

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

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        if let Some(content) = chunk.content {
            content_parts.push(content.clone());
            if !command.as_ref().map_or(false, |c| propose_commands.contains(&c.as_str())) {
                tx.send(SseEvent::TextDelta { content }).await.ok();
            }
        }

        if let Some(reasoning) = chunk.reasoning {
            reasoning_parts.push(reasoning);
        }

        if let Some(chunk_tool_calls) = chunk.tool_calls {
            for tc in chunk_tool_calls {
                // 这里需要从 ToolCall 中提取 index，但目前 ToolCall 没有 index 字段
                // 需要修改 ToolCall 结构或创建新的 StreamToolCall
                // 暂时简化处理：假设每次只有一个 tool_call
                if tc_accumulator.is_empty() {
                    tc_accumulator.insert(0, ToolCallBuilder {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    });
                } else {
                    // 追加参数（流式中 arguments 是分块的）
                    if let Some(builder) = tc_accumulator.get_mut(&0) {
                        builder.arguments.push_str(&tc.function.arguments);
                    }
                }
            }
        }

        if chunk.finish_reason == Some("tool_calls".to_string()) {
            final_tool_calls = Some(
                tc_accumulator.values().map(|b| b.build()).collect()
            );
            break;
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
        tool_calls: final_tool_calls,
    })
}

async fn execute_tool_calls(
    tool_calls: &[ToolCall],
    tx: &mpsc::Sender<SseEvent>,
) -> Result<Vec<String>> {
    let mut results = Vec::new();

    for tool_call in tool_calls {
        let function_name = &tool_call.function.name;
        let arguments: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;

        let result = match function_name.as_str() {
            "generate_chart" => {
                let chart_result = data_tools::tool_generate_chart(
                    arguments["chart_type"].as_str().unwrap_or("Bar_Chart"),
                    arguments["sql"].as_str().unwrap_or(""),
                    arguments["x_axis"].as_str().unwrap_or(""),
                    arguments["y_axis"].as_str().unwrap_or(""),
                    arguments["title"].as_str().unwrap_or(""),
                    arguments["subtitle"].as_str().unwrap_or(""),
                    arguments["legend"].as_str().unwrap_or(""),
                    arguments["tooltip"].as_str().unwrap_or(""),
                    arguments["data"].as_str().unwrap_or(""),
                )?;
                chart_result.warnings.join(", ")
            }
            "export_data" => {
                let export_result = data_tools::tool_export_data(
                    arguments["sql"].as_str().unwrap_or(""),
                    arguments["format"].as_str().unwrap_or(""),
                )?;
                tx.send(SseEvent::Download {
                    filename: export_result.filename,
                    content: export_result.content,
                }).await.ok();
                export_result.warnings.join(", ")
            }
            "propose_ppt_outline" => {
                let outline_result = data_tools::tool_propose_ppt_outline(
                    arguments["sql"].as_str().unwrap_or(""),
                    arguments["title"].as_str().unwrap_or(""),
                    arguments["subtitle"].as_str().unwrap_or(""),
                    arguments["sections"].as_str().unwrap_or(""),
                )?;
                tx.send(SseEvent::PptOutline {
                    title: outline_result.title,
                    sections: outline_result.sections,
                }).await.ok();
                "大纲已展示给用户，等待其通过按钮确认或修改。"
            }
            "handle_ppt_confirm" => {
                let confirm_result = data_tools::tool_handle_ppt_confirm(
                    arguments["sql"].as_str().unwrap_or(""),
                    arguments["title"].as_str().unwrap_or(""),
                    arguments["subtitle"].as_str().unwrap_or(""),
                    arguments["sections"].as_str().unwrap_or(""),
                )?;
                tx.send(SseEvent::PptHtml {
                    html: confirm_result.html,
                }).await.ok();
                confirm_result.warnings.join(", ")
            }
            _ => {
                return Err(anyhow!("Unknown tool: {}", function_name));
            }
        };
        
        results.push(result);
    }
    
    Ok(results)
}
