# LLM 客户端模块使用指南

## 📦 模块结构

```
src-tauri/src/llm/
├── mod.rs              # 模块入口，导出所有公共 API
├── client.rs           # LLMClient Trait 和核心类型定义
├── config.rs           # LLM 配置管理器
└── providers/
    ├── mod.rs          # Providers 模块导出
    ├── openai.rs       # OpenAI 客户端实现
    └── claude.rs       # Claude 客户端实现
```

## 🎯 核心组件

### 1. LLMClient Trait

所有 LLM 提供商必须实现的接口：

```rust
#[async_trait]
pub trait LLMClient: Send + Sync + Debug {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse>;
    fn model_name(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
}
```

### 2. Message 类型

```rust
// 创建消息
let system_msg = Message::system("You are a helpful assistant.");
let user_msg = Message::user("Hello!");
let assistant_msg = Message::assistant("Hi! How can I help you?");

// 消息角色
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}
```

### 3. ChatResponse 类型

```rust
pub struct ChatResponse {
    pub content: String,              // 回复内容
    pub reasoning: Option<String>,    // 推理链（可选）
    pub usage: Option<TokenUsage>,    // Token 使用统计
}

pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

## 🚀 快速开始

### 使用 OpenAI

```rust
use crate::llm::{OpenAIClient, LLMClient, Message};

// 创建客户端
let api_key = std::env::var("OPENAI_API_KEY")?;
let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string());

// 准备消息
let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::user("What is Rust?"),
];

// 调用 API
let response = client.chat(messages).await?;
println!("Response: {}", response.content);
println!("Tokens: {:?}", response.usage);
```

### 使用 Claude

```rust
use crate::llm::{ClaudeClient, LLMClient, Message};

// 创建客户端
let api_key = std::env::var("ANTHROPIC_API_KEY")?;
let client = ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string());

// 准备消息
let messages = vec![
    Message::system("You are a helpful assistant."),
    Message::user("What is Rust?"),
];

// 调用 API
let response = client.chat(messages).await?;
println!("Response: {}", response.content);
```

### 自定义 Base URL

适用于 Azure OpenAI、本地代理等场景：

```rust
let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string())
    .with_base_url("https://your-custom-endpoint.com/v1".to_string());
```

## 🧪 测试

### 运行单元测试

```bash
# 设置 API key
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# 运行测试
cd src-tauri
cargo test --lib llm -- --ignored --nocapture
```

### 使用 Tauri Command 测试

前端可以调用 `test_llm_chat` 命令进行测试：

```typescript
import { invoke } from '@tauri-apps/api/core'

const result = await invoke('test_llm_chat', {
  request: {
    provider: 'openai',  // 或 'claude'
    message: 'Hello!'
  }
})

console.log(result.content)
console.log(result.model)
console.log(result.tokens)
```

## ⚙️ 配置管理

使用 `LLMConfigManager` 管理多个 LLM 配置：

```rust
use crate::llm::LLMConfigManager;

let mut manager = LLMConfigManager::new();

// 从文件加载配置
manager.load_from_file()?;

// 获取配置
if let Some(config) = manager.get_config("openai") {
    println!("API Key: {}", config.api_key);
    println!("Model: {:?}", config.model);
}

// 保存配置
manager.save_to_file()?;
```

配置文件格式 (`LLM/llm_config.json`)：

```json
{
  "openai": {
    "provider": "openai",
    "api_key": "sk-...",
    "base_url": null,
    "model": "gpt-4o-mini",
    "enabled": true,
    "is_custom": false,
    "context_window": 128000,
    "max_output_tokens": 16384,
    "enable_thinking": false
  }
}
```

## 📝 扩展新的 LLM 提供商

要添加新的 LLM 提供商（如 DeepSeek）：

1. 创建新文件 `src/llm/providers/deepseek.rs`
2. 实现 `LLMClient` Trait
3. 在 `providers/mod.rs` 中导出
4. 在 `llm/mod.rs` 中重新导出

示例：

```rust
// src/llm/providers/deepseek.rs
use crate::llm::client::*;
use async_trait::async_trait;

#[derive(Debug)]
pub struct DeepSeekClient {
    // ... 实现细节
}

#[async_trait]
impl LLMClient for DeepSeekClient {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse> {
        // 实现 API 调用逻辑
        todo!()
    }
    
    fn model_name(&self) -> &str {
        "deepseek-chat"
    }
}
```

## 🔧 故障排除

### 问题 1: 编译错误 "unresolved import"

**解决：** 确保在 `llm/mod.rs` 中正确导出了新的类型。

### 问题 2: 运行时错误 "API key not found"

**解决：** 设置环境变量：
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### 问题 3: API 调用失败

**解决：** 
- 检查网络连接
- 验证 API key 是否有效
- 查看日志输出（使用 `tracing`）

## 📊 性能优化建议

1. **复用 Client 实例**：不要每次请求都创建新的 Client
2. **连接池**：reqwest Client 内部已实现连接池
3. **异步并发**：使用 `tokio::spawn` 并行处理多个请求
4. **缓存响应**：对相同请求缓存结果

## 🔐 安全注意事项

1. **不要硬编码 API key**：始终从环境变量或配置文件读取
2. **配置文件权限**：确保 `llm_config.json` 文件权限正确
3. **HTTPS**：所有 API 调用必须使用 HTTPS
4. **Token 限制**：设置合理的 `max_output_tokens` 防止超额费用

## 📚 相关文档

- [快速启动指南](../QUICK_START_MIGRATION.md)
- [详细迁移计划](../PYTHON_TO_RUST_MIGRATION_PLAN.md)
- [架构对比](../ARCHITECTURE_COMPARISON.md)

---

*最后更新：2026-05-16*
