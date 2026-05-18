pub mod openai;
// pub mod claude;  // ✅ 暂时禁用 Claude，专注于调试 OpenAI

pub use openai::OpenAIClient;
// pub use claude::ClaudeClient;  // ✅ 暂时禁用