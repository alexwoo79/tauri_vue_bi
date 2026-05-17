# 🚀 LLM 客户端模块 - 快速参考

## 📦 已创建的文件

```
✅ src-tauri/src/llm/mod.rs           (7 行)
✅ src-tauri/src/llm/client.rs        (82 行)
✅ src-tauri/src/llm/config.rs        (73 行)
✅ src-tauri/src/llm/providers/mod.rs (5 行)
✅ src-tauri/src/llm/providers/openai.rs (164 行)
✅ src-tauri/src/llm/providers/claude.rs (179 行)
✅ src-tauri/src/commands/llm_test.rs (50 行)
```

## 🔑 核心 API

### 导入
```rust
use crate::llm::{LLMClient, Message, OpenAIClient, ClaudeClient};
```

### 创建客户端
```rust
// OpenAI
let client = OpenAIClient::new(api_key, "gpt-4o-mini".to_string());

// Claude
let client = ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string());
```

### 调用聊天
```rust
let messages = vec![
    Message::system("You are helpful."),
    Message::user("Hello!"),
];

let response = client.chat(messages).await?;
println!("{}", response.content);
```

## 🧪 测试命令

```bash
# 设置 API key
export OPENAI_API_KEY="sk-..."

# 运行测试
cd src-tauri
cargo test --lib llm -- --ignored --nocapture
```

## 📝 前端调用

```typescript
import { invoke } from '@tauri-apps/api/core'

const result = await invoke('test_llm_chat', {
  request: {
    provider: 'openai',
    message: 'Hello!'
  }
})
```

## ⚙️ 配置管理

```rust
let mut manager = LLMConfigManager::new();
manager.load_from_file()?;
let config = manager.get_config("openai");
```

## 🔗 相关文档

- [使用指南](./LLM_MODULE_USAGE.md)
- [完成总结](./STEP1_COMPLETE.md)
- [快速启动](./QUICK_START_MIGRATION.md)

---

*创建时间：2026-05-16*
