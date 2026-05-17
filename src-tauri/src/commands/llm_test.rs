use crate::llm::{LLMClient, Message, OpenAIClient};//、✅ 暂时禁用 Claude，专注于调试 OpenAI  
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

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
    // 从环境变量获取 API key
    let api_key = match request.provider.as_str() {
        "openai" => std::env::var("OPENAI_API_KEY")
            .map_err(|_| "Please set OPENAI_API_KEY environment variable".to_string())?,
        // "claude" => std::env::var("ANTHROPIC_API_KEY")
        //     .map_err(|_| "Please set ANTHROPIC_API_KEY environment variable".to_string())?,
        _ => return Err(format!("Unsupported provider: {}", request.provider)),
    };

    // 创建客户端
    let client: Box<dyn LLMClient> = match request.provider.as_str() {
        "openai" => Box::new(OpenAIClient::new(api_key, "gpt-4o-mini".to_string())),
        // "claude" => Box::new(ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string())),
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

/// 流式聊天命令（用于前端实时显示）
#[tauri::command]
pub async fn test_llm_chat_stream(
    app_handle: tauri::AppHandle,
    session_id: String,
    request: TestChatRequest,
) -> Result<(), String> {
    // 从环境变量获取 API key
    let api_key = match request.provider.as_str() {
        "openai" => std::env::var("OPENAI_API_KEY")
            .map_err(|_| "Please set OPENAI_API_KEY environment variable".to_string())?,
        "claude" => std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "Please set ANTHROPIC_API_KEY environment variable".to_string())?,
        _ => return Err(format!("Unsupported provider: {}", request.provider)),
    };

    // 创建客户端
    let client: Box<dyn LLMClient> = match request.provider.as_str() {
        "openai" => Box::new(OpenAIClient::new(api_key, "gpt-4o-mini".to_string())),
        // "claude" => Box::new(ClaudeClient::new(api_key, "claude-3-5-haiku-20241022".to_string())),
        _ => return Err(format!("Unsupported provider: {}", request.provider)),
    };

    // 准备消息
    let messages = vec![Message::user(request.message)];

    // 获取流
    let mut stream = client
        .chat_stream(messages)
        .await
        .map_err(|e| format!("LLM stream failed: {}", e))?;

    // 逐块发送事件到前端
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // 发送内容增量
                if let Some(content) = &chunk.content {
                    let _ = app_handle.emit(
                        "llm-stream-chunk",
                        serde_json::json!({
                            "session_id": session_id,
                            "content": content,
                            "finish_reason": chunk.finish_reason
                        }),
                    );
                }

                // 如果结束，跳出循环
                if chunk.finish_reason.is_some() {
                    break;
                }
            }
            Err(e) => {
                // 发送错误
                let _ = app_handle.emit(
                    "llm-stream-error",
                    serde_json::json!({
                        "session_id": session_id,
                        "error": e.to_string()
                    }),
                );
                break;
            }
        }
    }

    // 发送完成事件
    let _ = app_handle.emit(
        "llm-stream-done",
        serde_json::json!({
            "session_id": session_id
        }),
    );

    Ok(())
}
