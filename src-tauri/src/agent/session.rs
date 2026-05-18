// src-tauri/src/agent/session.rs
//
// 会话管理模块 - Session Management
//
// 提供以下功能：
// - 创建、删除、列出会话
// - 会话历史管理
// - 会话持久化（可选）

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use uuid::Uuid;

/// 消息角色枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }
}

/// 兼容旧版本的 ChatMessage（用于 agent_chat.rs）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::agent::llm::ToolCall>>,
}

/// 单条消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::agent::llm::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            reasoning_content: None,
            tool_calls: None,
            timestamp: Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

/// 聊天会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub name: String,
    pub title: String,
    pub model_id: String,
    pub messages: Vec<Message>,
    pub created_at: String,
    pub updated_at: String,
}

impl ChatSession {
    pub fn new(name: Option<String>) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let name_str = name.unwrap_or_else(|| "New Session".to_string());
        Self {
            id: Uuid::new_v4().to_string(),
            name: name_str.clone(),
            title: name_str,
            model_id: "".to_string(),
            messages: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn with_name(name: &str) -> Self {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            title: name.to_string(),
            model_id: "".to_string(),
            messages: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.add_message(Message::new(MessageRole::User, content.to_string()));
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.add_message(Message::new(MessageRole::Assistant, content.to_string()));
    }

    pub fn add_assistant_message_with_tools(
        &mut self,
        content: &str,
        tool_calls: Vec<crate::agent::llm::ToolCall>,
    ) {
        self.add_assistant_message_with_tools_and_reasoning(content, None, tool_calls);
    }
    
    /// ✅ 新增：添加包含工具调用和 reasoning_content 的助手消息（支持 thinking 模型）
    pub fn add_assistant_message_with_tools_and_reasoning(
        &mut self,
        content: &str,
        reasoning_content: Option<String>,
        tool_calls: Vec<crate::agent::llm::ToolCall>,
    ) {
        let mut message = Message::new(MessageRole::Assistant, content.to_string());
        message.reasoning_content = reasoning_content;
        message.tool_calls = Some(tool_calls);
        self.add_message(message);
    }

    pub fn add_system_message(&mut self, content: &str) {
        self.add_message(Message::new(MessageRole::System, content.to_string()));
    }

    pub fn get_messages_for_llm(&self) -> Vec<crate::agent::llm::Message> {
        self.messages
            .iter()
            .map(|msg| {
                crate::agent::llm::Message {
                    role: match msg.role {
                        MessageRole::User => crate::agent::llm::MessageRole::User,
                        MessageRole::Assistant => crate::agent::llm::MessageRole::Assistant,
                        MessageRole::System => crate::agent::llm::MessageRole::System,
                    },
                    content: msg.content.clone(),
                    reasoning_content: msg.reasoning_content.clone(),
                    tool_calls: msg.tool_calls.clone(),
                    tool_call_id: None,
                }
            })
            .collect()
    }
}

/// 会话管理器
pub struct SessionManager {
    sessions: HashMap<String, ChatSession>,
    current_session_id: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            current_session_id: None,
        }
    }

    pub fn get_sessions_mut(&mut self) -> &mut HashMap<String, ChatSession> {
        &mut self.sessions
    }

    pub fn create_session(&mut self, name: Option<String>) -> String {
        let session = ChatSession::new(name);
        let session_id = session.id.clone();
        self.sessions.insert(session_id.clone(), session);
        self.current_session_id = Some(session_id.clone());
        session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<&ChatSession> {
        self.sessions.get(session_id)
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut ChatSession> {
        self.sessions.get_mut(session_id)
    }

    pub fn delete_session(&mut self, session_id: &str) -> bool {
        let deleted = self.sessions.remove(session_id).is_some();
        if self.current_session_id.as_deref() == Some(session_id) {
            self.current_session_id = None;
        }
        deleted
    }

    pub fn list_sessions(&self) -> Vec<ChatSession> {
        self.sessions.values().cloned().collect()
    }

    pub fn set_current_session(&mut self, session_id: &str) -> bool {
        if self.sessions.contains_key(session_id) {
            self.current_session_id = Some(session_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn get_current_session(&self) -> Option<&ChatSession> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.sessions.get(id))
    }

    pub fn get_current_session_mut(&mut self) -> Option<&mut ChatSession> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.sessions.get_mut(id))
    }
}

/// 全局会话管理器
pub static GLOBAL_SESSION_MANAGER: Lazy<Mutex<SessionManager>> = 
    Lazy::new(|| Mutex::new(SessionManager::new()));

use once_cell::sync::Lazy;