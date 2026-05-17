# Python → Rust 迁移快速启动指南

> 🚀 本文档提供从零开始的实操步骤，帮助你快速启动迁移工作。

---

## 📋 前置准备

### 1. 环境检查

```bash
# 确认 Rust 工具链
rustc --version    # 应该 >= 1.77.2
cargo --version

# 确认 Tauri CLI
npm list -g @tauri-apps/cli

# 确认 Node.js
node --version     # 应该 >= 20
```

### 2. 项目结构预览

```
tauri-vue-bi/
├── Data-Analysis-Agent/      # 🐍 Python 代码（待迁移）
│   ├── agent/                # AI Agent 核心
│   ├── api/                  # Flask API
│   ├── LLM/                  # LLM 配置
│   └── Function/             # 数据分析功能
│
├── src-tauri/                # 🦀 Rust 代码（目标）
│   ├── src/
│   │   ├── commands/         # Tauri 命令
│   │   ├── state.rs          # 全局状态
│   │   └── types.rs          # 类型定义
│   └── Cargo.toml            # Rust 依赖
│
└── PYTHON_TO_RUST_MIGRATION_PLAN.md  # 详细迁移计划
```

---

## 🎯 第一步：创建 LLM 客户端模块（30 分钟）

这是整个系统的核心抽象，且相对独立，是最佳的起点。

### 1.1 添加依赖

```bash
cd src-tauri
cargo add reqwest serde serde_json anyhow async-trait tokio
cargo add async-stream futures tracing thiserror
```

在 `Cargo.toml` 中确认：

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
async-stream = "0.3"
futures = "0.3"
tracing = "0.1"
```

### 1.2 创建模块结构

```bash
mkdir -p src/llm/providers
```

创建文件：
- `src/llm/mod.rs`
- `src/llm/client.rs`
- `src/llm/providers/mod.rs`
- `src/llm/providers/openai.rs`

### 1.3 实现核心 Trait

**文件：`src/llm/client.rs`**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>, // 推理链（DeepSeek R1 / Claude 3.7+）
    pub usage: Option<TokenUsage>,
}

/// LLM 客户端 Trait
#[async_trait]
pub trait LLMClient: Send + Sync + Debug {
    /// 同步聊天
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse>;

    /// 获取模型名称
    fn model_name(&self) -> &str;

    /// 获取上下文窗口大小（tokens）
    fn context_window(&self) -> usize {
        4096 // 默认值
    }

    /// 获取最大输出长度（tokens）
    fn max_output_tokens(&self) -> usize {
        2048 // 默认值
    }
}
```

**文件：`src/llm/mod.rs`**

```rust
pub mod client;
pub mod providers;

pub use client::*;
```

**文件：`src/llm/providers/mod.rs`**

```rust
pub mod openai;

pub use openai::OpenAIClient;
```

### 1.4 实现 OpenAI 客户端

**文件：`src/llm/providers/openai.rs`**

```rust
use crate::llm::client::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
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
            })
            .collect();

        let request = ChatRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: Some(self.max_output_tokens),
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
        })
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
```

### 1.5 编写测试

在 `src/llm/providers/openai.rs` 底部添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

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
```

### 1.6 运行测试

```bash
# 设置 API key（替换为你的真实 key）
export OPENAI_API_KEY="sk-..."

# 运行测试
cargo test --lib llm::providers::openai::tests -- --ignored --nocapture
```

预期输出：
```
Running unittests src/lib.rs (target/debug/deps/...)

Response: Hello! I'm an AI assistant...
Tokens used: 42

test llm::providers::openai::tests::test_openai_chat ... ok
```

---

## 🎯 第二步：实现 Claude 客户端（20 分钟）

Claude 的实现与 OpenAI 类似，但 API 格式略有不同。

### 2.1 创建 Claude 文件

**文件：`src/llm/providers/claude.rs`**

```rust
use crate::llm::client::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize)]
struct ClaudeTokenUsage {
    input_tokens: u32,
    output_tokens: u32,
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
        };

        let mut req_builder = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request);

        // 如果有 system prompt，添加到 header
        if let Some(system) = system_prompt {
            req_builder = req_builder.header("anthropic-beta", "prompt-caching-2024-07-31");
            // 注意：Claude 的 system prompt 通过 header 传递有特殊限制
            // 这里简化处理，实际生产环境需要更复杂的逻辑
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
        })
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
```

### 2.2 更新 providers/mod.rs

```rust
pub mod openai;
pub mod claude;  // 新增

pub use openai::OpenAIClient;
pub use claude::ClaudeClient;  // 新增
```

---

## 🎯 第三步：创建配置管理器（15 分钟）

### 3.1 定义配置结构

**文件：`src/llm/config.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    pub is_custom: bool,
    pub context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub enable_thinking: bool,
}

pub struct LLMConfigManager {
    configs: HashMap<String, LLMConfig>,
    config_dir: PathBuf,
}

impl LLMConfigManager {
    pub fn new() -> Self {
        let config_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("LLM");

        fs::create_dir_all(&config_dir).ok();

        Self {
            configs: HashMap::new(),
            config_dir,
        }
    }

    pub fn load_from_file(&mut self) -> anyhow::Result<()> {
        let config_file = self.config_dir.join("llm_config.json");

        if !config_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&config_file)?;
        let configs: HashMap<String, LLMConfig> = serde_json::from_str(&content)?;

        self.configs = configs;
        Ok(())
    }

    pub fn save_to_file(&self) -> anyhow::Result<()> {
        let config_file = self.config_dir.join("llm_config.json");
        let content = serde_json::to_string_pretty(&self.configs)?;
        fs::write(&config_file, content)?;
        Ok(())
    }

    pub fn get_config(&self, provider: &str) -> Option<&LLMConfig> {
        self.configs.get(provider)
    }

    pub fn set_config(&mut self, provider: String, config: LLMConfig) {
        self.configs.insert(provider, config);
    }

    pub fn get_enabled_configs(&self) -> Vec<&LLMConfig> {
        self.configs.values().filter(|c| c.enabled).collect()
    }

    pub fn get_default_provider(&self) -> Option<String> {
        self.configs
            .iter()
            .find(|(_, c)| c.enabled)
            .map(|(k, _)| k.clone())
    }
}
```

### 3.2 更新 llm/mod.rs

```rust
pub mod client;
pub mod providers;
pub mod config;  // 新增

pub use client::*;
pub use config::LLMConfigManager;
```

---

## 🎯 第四步：集成到 Tauri Commands（30 分钟）

### 4.1 创建新的 command 文件

**文件：`src/commands/llm_test.rs`**

```rust
use crate::llm::{LLMClient, Message, OpenAIClient, ClaudeClient, LLMConfigManager};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct TestChatRequest {
    pub provider: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TestChatResponse {
    pub content: String,
    pub model: String,
    pub tokens: Option<u32>,
}

#[tauri::command]
pub async fn test_llm_chat(
    request: TestChatRequest,
) -> Result<TestChatResponse, String> {
    // 从环境变量或配置文件获取 API key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "Please set OPENAI_API_KEY environment variable".to_string())?;

    // 创建客户端
    let client: Box<dyn LLMClient> = match request.provider.as_str() {
        "openai" => Box::new(OpenAIClient::new(api_key, "gpt-4o-mini".to_string())),
        "claude" => {
            let claude_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| "Please set ANTHROPIC_API_KEY environment variable".to_string())?;
            Box::new(ClaudeClient::new(claude_key, "claude-3-5-haiku-20241022".to_string()))
        }
        _ => return Err(format!("Unsupported provider: {}", request.provider)),
    };

    // 调用 LLM
    let messages = vec![Message::user(request.message)];
    let response = client
        .chat(messages)
        .await
        .map_err(|e| format!("LLM call failed: {}", e))?;

    Ok(TestChatResponse {
        content: response.content,
        model: client.model_name().to_string(),
        tokens: response.usage.map(|u| u.total_tokens),
    })
}
```

### 4.2 注册 command

**文件：`src/commands/mod.rs`**

```rust
pub mod chart;
pub mod clean;
// ... 其他模块
pub mod llm_test;  // 新增

// 导出所有 commands
pub use llm_test::test_llm_chat;  // 新增
```

**文件：`src/lib.rs`**

```rust
use crate::commands::{
    // ... 其他 imports
    llm_test::test_llm_chat,  // 新增
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ... 加载数据集 registry

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            // ... 其他 commands
            test_llm_chat,  // 新增
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 4.3 前端调用示例

**文件：`src/views/TestLLM.vue`**（新建）

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

const provider = ref('openai')
const message = ref('Hello!')
const response = ref('')
const loading = ref(false)

async function sendTest() {
  loading.value = true
  try {
    const result = await invoke('test_llm_chat', {
      request: {
        provider: provider.value,
        message: message.value
      }
    })
    
    response.value = result.content
    ElMessage.success(`Tokens: ${result.tokens}`)
  } catch (error) {
    ElMessage.error(`Error: ${error}`)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="test-llm">
    <h2>LLM Test</h2>
    
    <el-select v-model="provider">
      <el-option label="OpenAI" value="openai" />
      <el-option label="Claude" value="claude" />
    </el-select>
    
    <el-input v-model="message" placeholder="Enter message" />
    
    <el-button type="primary" @click="sendTest" :loading="loading">
      Send
    </el-button>
    
    <div v-if="response" class="response">
      <pre>{{ response }}</pre>
    </div>
  </div>
</template>

<style scoped>
.test-llm {
  padding: 20px;
  max-width: 800px;
  margin: 0 auto;
}

.response {
  margin-top: 20px;
  padding: 15px;
  background: #f5f5f5;
  border-radius: 4px;
  white-space: pre-wrap;
}
</style>
```

---

## ✅ 验证清单

完成以上四步后，你应该能够：

- [ ] 编译成功：`cargo build`
- [ ] 单元测试通过：`cargo test --lib llm`
- [ ] Tauri 应用启动：`npm run tauri dev`
- [ ] 前端调用成功：在测试页面发送消息并收到回复

---

## 🎓 下一步学习路径

完成基础 LLM 客户端后，按以下顺序继续：

1. **实现流式响应**（2-3 小时）
   - 学习 `async-stream` crate
   - 实现 `chat_stream` 方法
   - 前端处理 SSE

2. **实现 DeepSeek 客户端**（1 小时）
   - 复用 OpenAI 客户端代码
   - 修改 baseURL 和模型名

3. **Agent 状态机**（1-2 天）
   - 定义 Agent 结构
   - 实现迭代循环
   - 管理对话历史

4. **工具系统**（2-3 天）
   - 定义 Tool Trait
   - 实现数据查询工具
   - 实现图表生成工具

5. **Axum Web 服务器**（2-3 天）
   - 创建路由器
   - 实现 SSE endpoints
   - 迁移 Flask API

---

## 💡 常见问题

### Q1: 编译错误 "cannot find trait `LLMClient`"

**解决：** 确保在 `src/lib.rs` 中声明了 `llm` 模块：

```rust
pub mod llm;  // 添加这行
```

### Q2: 运行时错误 "OPENAI_API_KEY not found"

**解决：** 设置环境变量：

```bash
# macOS/Linux
export OPENAI_API_KEY="sk-..."

# Windows (PowerShell)
$env:OPENAI_API_KEY="sk-..."
```

或在 `.env` 文件中配置（需要使用 `dotenv` crate）。

### Q3: 如何调试异步代码？

**解决：** 使用 `tracing` 而非 `println!`：

```rust
use tracing::{info, debug, error};

info!(key = value, "Message");
```

启动时初始化 tracing：

```rust
tracing_subscriber::fmt::init();
```

### Q4: 如何处理 API 限流？

**解决：** 添加重试机制：

```rust
use tokio::time::{sleep, Duration};

async fn chat_with_retry(&self, messages: Vec<Message>) -> anyhow::Result<ChatResponse> {
    for attempt in 1..=3 {
        match self.chat(messages.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) if attempt < 3 => {
                tracing::warn!(attempt, "Retry after error: {}", e);
                sleep(Duration::from_secs(2 * attempt as u64)).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

---

## 📚 参考资源

- **完整迁移计划：** [PYTHON_TO_RUST_MIGRATION_PLAN.md](./PYTHON_TO_RUST_MIGRATION_PLAN.md)
- **Rust Book:** https://doc.rust-lang.org/book/
- **Tokio Tutorial:** https://tokio.rs/tokio/tutorial
- **Axum Examples:** https://github.com/tokio-rs/axum/tree/main/examples

---

**祝你编码愉快！** 🚀

*如有问题，请查阅详细迁移计划或提出具体问题。*
