# OpenAI 工作流调试计划 - 2026-05-17

## 📅 日期: 2026-05-17

---

## 🎯 当前目标

**暂时禁用 Claude 客户端，专注于调试 OpenAI 的完整工作流**

---

## ✅ 已完成的修改

### 1. 禁用 Claude 模块

#### `src-tauri/src/llm/providers/mod.rs`
```rust
pub mod openai;
// pub mod claude;  // ✅ 暂时禁用 Claude，专注于调试 OpenAI

pub use openai::OpenAIClient;
// pub use claude::ClaudeClient;  // ✅ 暂时禁用
```

#### `src-tauri/src/commands/agent_chat.rs`
```rust
use crate::llm::{LLMClient, Message, MessageRole, OpenAIClient};
// use crate::llm::ClaudeClient;  // ✅ 暂时禁用 Claude

// ...

let provider_lc = provider.to_lowercase();
// ✅ 暂时禁用 Claude，统一使用 OpenAI
let model_name = model.unwrap_or("gpt-4o-mini");

// if provider_lc == "claude" {
//     return Ok(Box::new(ClaudeClient::new(key, model_name.to_string())));
// }

let mut client = OpenAIClient::new(key, model_name.to_string());
```

#### `src-tauri/src/agent/state_machine.rs`
```rust
// ✅ 暂时禁用 Claude thinking mode，统一使用流式
// let response = if enable_thinking && model.starts_with("claude") {
//     // Claude thinking mode: 非流式
//     client.chat(messages.clone()).await?
// } else {
    // 流式模式
    let stream = client.chat_stream(messages.clone()).await?;
    Self::process_stream(
        stream,
        &tx,
        &mut collected_text,
        &mut all_reasoning,
        &params.command,
        &propose_commands,
    )
    .await?
// };
```

---

## 🔍 OpenAI 工作流调试重点

### P0 - 核心功能验证

#### 1. 工具调用解析 ✅ (已完成)
- [x] `StreamToolCall` 数据结构
- [x] `ToolCallBuilder` 按 index 累积参数
- [x] SSE 循环中拼接分块的 name 和 arguments
- [x] 检测 `finish_reason == "tool_calls"` 发送完整列表

**测试方法**:
```javascript
import { invoke } from '@tauri-apps/api/core'

const session = await invoke('create_session', { modelId: 'gpt-4o' })

await invoke('chat_stream', {
  sessionId: session.session_id,
  userMessage: '/chart 销售趋势',
  command: 'chart'
})

// 监听事件
const unlisten = await listen('sse-event', (event) => {
  console.log('Event:', event.payload)
  
  if (event.payload.type === 'tool_start') {
    console.log('✅ 工具调用被检测到！', event.payload.tool)
  }
  
  if (event.payload.type === 'done') {
    unlisten()
  }
})
```

**预期结果**:
- ✅ 看到 `tool_start` 事件
- ✅ 看到工具执行结果 (`tool_result`)
- ✅ LLM 基于工具结果继续推理或返回最终答案

---

#### 2. Agent 状态机循环 ⚠️ (待验证)
**关键流程**:
```
用户消息 → LLM → 工具调用 → 执行工具 → 工具结果 → LLM → 最终响应
```

**需要验证的点**:
1. ✅ 工具调用能被正确解析（P0 已修复）
2. ⚠️ 工具执行后是否正确返回给 LLM
3. ⚠️ LLM 是否能基于工具结果生成最终响应
4. ⚠️ 整个循环是否正常结束

**调试日志点**:
```rust
// state_machine.rs 中的关键位置
println!("🔧 Executing tool: {:?}", tool_call.function.name);
println!("📊 Tool result: {:?}", result);
println!("🤖 Sending tool result to LLM");
```

---

#### 3. SSE 事件推送 ⚠️ (待优化)
**当前问题**: 
- 可能是收集后一次性返回，而非真正的实时推送

**需要验证**:
1. ⚠️ 文本增量是否实时推送 (`TextDelta` 事件)
2. ⚠️ 工具调用事件是否及时推送 (`ToolStart`, `ToolEnd`)
3. ⚠️ 图表生成事件是否及时推送 (`ChartGenerated`)

**前端监听**:
```javascript
const unlisten = await listen('sse-event', (event) => {
  const payload = event.payload
  
  switch (payload.type) {
    case 'text_delta':
      console.log('📝 文本增量:', payload.content)
      break
    case 'tool_start':
      console.log('🔧 工具开始:', payload.tool)
      break
    case 'tool_end':
      console.log('✅ 工具结束:', payload.result)
      break
    case 'chart_generated':
      console.log('📊 图表生成:', payload.chart)
      break
    case 'done':
      console.log('🏁 对话完成')
      break
  }
})
```

---

### P1 - 功能完整性

#### 4. 图表收集机制 ❌ (缺失)
**Python 实现**:
```python
pending_charts = []  # 在 run_loop 中维护

# 执行工具时收集图表
if chart_html:
    pending_charts.append(chart_html)

# 最终响应前统一发送
for chart in pending_charts:
    yield SseEvent.chart_generated(chart)
```

**Rust 当前状态**: ❌ 完全缺失

**需要添加**:
```rust
// state_machine.rs run_loop 函数中
let mut pending_charts: Vec<String> = Vec::new();

// execute_tool_calls 中收集
if let Some(chart_html) = generate_chart(...) {
    pending_charts.push(chart_html);
}

// 最终响应前发送
for chart in pending_charts {
    tx.send(SseEvent::ChartGenerated { html: chart }).await.ok();
}
```

---

#### 5. 大纲提议流程 ⚠️ (不完整)
**Python 实现**:
```python
outline_proposed = False

# propose 工具执行后
if command == "propose":
    outline_proposed = True
    return  # 立即返回，等待用户确认

# 主循环检查
if outline_proposed:
    return  # 中断循环
```

**Rust 当前状态**: ⚠️ 有快速路径但缺少标志

**需要完善**:
```rust
// run_loop 中添加标志
let mut outline_proposed = false;

// execute_tool_calls 中设置
if tool_name == "propose_ppt_outline" || ... {
    outline_proposed = true;
}

// 主循环检查
if outline_proposed {
    break;  // 中断循环，等待用户确认
}
```

---

## 📋 调试步骤

### Step 1: 编译验证 ✅
```bash
cd /Users/alex/Documents/github/tauri-vue-bi/src-tauri
cargo check --lib
```

**预期**: 无错误，可能有少量警告

---

### Step 2: 启动开发服务器
```bash
cd /Users/alex/Documents/github/tauri-vue-bi
npm run tauri dev
```

**观察**:
- 编译是否成功
- 应用是否正常启动
- 控制台是否有错误

---

### Step 3: 基础聊天测试
在前端打开 Agent Chat 页面，发送简单消息：

```
你好，请介绍一下自己
```

**预期**:
- ✅ 看到流式文本输出
- ✅ 最终显示完整回复
- ✅ 没有错误

---

### Step 4: 工具调用测试
发送需要工具调用的消息：

```
/chart 销售趋势
```

**预期**:
- ✅ 看到 `tool_start` 事件
- ✅ 看到工具执行过程
- ✅ 看到 `tool_end` 事件
- ✅ LLM 基于工具结果生成响应
- ✅ 最终显示图表或分析结果

**如果失败**:
1. 检查浏览器控制台的事件日志
2. 检查 Rust 后端的日志输出
3. 确认 OpenAI API Key 是否正确配置

---

### Step 5: 导出功能测试
发送导出命令：

```
/export excel
```

**预期**:
- ✅ 看到 `tool_start` 事件
- ✅ 看到 Excel 文件生成
- ✅ 提供下载链接

---

## 🐛 常见问题排查

### 问题 1: 看不到 tool_start 事件
**可能原因**:
- OpenAI API Key 未配置
- 模型不支持工具调用
- 工具定义不正确

**解决方法**:
1. 检查 `.env` 或配置文件中的 API Key
2. 确认使用的是支持工具的模型（如 gpt-4o, gpt-4o-mini）
3. 检查 `tools_schema.rs` 中的工具定义

---

### 问题 2: 工具执行后无响应
**可能原因**:
- 工具执行失败但未返回错误
- 工具结果未正确发送给 LLM
- LLM 未收到工具结果

**解决方法**:
1. 在 `execute_tool_calls` 中添加日志
2. 检查 `ChatResponse` 的 `tool_calls` 字段
3. 确认消息历史中包含工具结果

---

### 问题 3: SSE 事件不实时
**可能原因**:
- 使用了缓冲模式而非真正的流式
- 前端监听器未及时更新 UI

**解决方法**:
1. 检查 `chat_stream` 是否真正逐块发送
2. 在前端每个事件中更新 UI
3. 添加时间戳验证延迟

---

## 📊 进度跟踪

| 任务 | 状态 | 备注 |
|------|------|------|
| 禁用 Claude 模块 | ✅ 完成 | 已注释所有相关代码 |
| OpenAI 工具调用解析 | ✅ 完成 | P0 修复 |
| 编译通过 | ⏳ 进行中 | 等待 cargo check |
| 基础聊天测试 | ⏸️ 待测试 | 需启动应用 |
| 工具调用测试 | ⏸️ 待测试 | 需验证完整流程 |
| SSE 实时性验证 | ⏸️ 待测试 | 需前端配合 |
| 图表收集机制 | ❌ 待实现 | P1 任务 |
| 大纲提议流程 | ⚠️ 待完善 | P1 任务 |

---

## 🎯 下一步行动

1. **等待编译完成** - 确认禁用 Claude 后无错误
2. **启动应用** - `npm run tauri dev`
3. **基础聊天测试** - 验证流式文本输出
4. **工具调用测试** - 验证完整 ReAct 循环
5. **根据测试结果决定**:
   - 如果成功 → 继续 P1 功能完善
   - 如果失败 → 深入调试具体问题

---

**最后更新**: 2026-05-17  
**状态**: 🔧 **Claude 已禁用，等待编译验证**  
**预计调试时间**: 2-4 小时（取决于问题复杂度）
