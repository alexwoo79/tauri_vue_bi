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
    
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => MessageRole::User,
        }
    }
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,  // "user" or "assistant" or "tool"
    pub content: String,
    pub timestamp: u64,
    /// ✅ 新增：支持 reasoning_content（用于 thinking 模型）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// ✅ 新增：支持 tool_call_id（用于工具调用结果）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// ✅ 新增：支持 tool_calls（用于 assistant 的工具调用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::llm::ToolCall>>,
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub model_id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
    pub cancel_requested: bool,  // ✅ 添加取消请求标志
}

impl ChatSession {
    /// 创建新会话
    pub fn new(model_id: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: Uuid::new_v4().to_string(),
            title: "新会话".to_string(),
            model_id: model_id.to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            cancel_requested: false,  // ✅ 初始化取消标志
        }
    }
    
    /// 添加用户消息
    pub fn add_user_message(&mut self, content: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: now,
            reasoning_content: None,  // ✅ 用户消息不包含 reasoning_content
            tool_call_id: None,  // ✅ 用户消息不包含 tool_call_id
            tool_calls: None,  // ✅ 用户消息不包含 tool_calls
        });
        
        self.updated_at = now;
        
        // 如果标题还是默认的，用第一条消息作为标题
        if self.title == "新会话" && self.messages.len() == 1 {
            self.title = content.chars().take(20).collect::<String>();
        }
    }
    
    /// ✅ 新增：添加用户消息的别名方法（兼容旧 API）
    pub fn add_user(&mut self, content: &str) {
        self.add_user_message(content);
    }
    
    /// 添加助手消息
    pub fn add_assistant_message(&mut self, content: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: now,
            reasoning_content: None,  // ✅ 普通助手消息不包含 reasoning_content
            tool_call_id: None,  // ✅ 助手消息默认不包含 tool_call_id
            tool_calls: None,  // ✅ 助手消息默认不包含 tool_calls
        });
        
        self.updated_at = now;
    }
    
    /// ✅ 新增：添加包含工具调用的助手消息
    pub fn add_assistant_message_with_tools(
        &mut self,
        content: &str,
        tool_calls: Vec<crate::llm::ToolCall>,
    ) {
        self.add_assistant_message_with_tools_and_reasoning(content, None, tool_calls);
    }
    
    /// ✅ 新增：添加包含工具调用和 reasoning_content 的助手消息（支持 thinking 模型）
    pub fn add_assistant_message_with_tools_and_reasoning(
        &mut self,
        content: &str,
        reasoning_content: Option<String>,
        tool_calls: Vec<crate::llm::ToolCall>,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: now,
            reasoning_content,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        });
        
        self.updated_at = now;
    }
    
    /// ✅ 新增：添加工具调用结果消息
    pub fn add_tool_message(&mut self, content: &str, tool_call_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.messages.push(ChatMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            timestamp: now,
            reasoning_content: None,  // ✅ tool 消息不包含 reasoning_content
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
        });
        
        self.updated_at = now;
    }
    
    /// ✅ 新增：添加助手消息的别名方法（兼容旧 API）
    pub fn add_assistant(&mut self, content: &str) {
        self.add_assistant_message(content);
    }
    
    /// 清除历史
    pub fn clear_history(&mut self) {
        self.messages.clear();
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// 获取最近的历史消息（最多 n 条）
    pub fn recent_history(&self, n: usize) -> Vec<&ChatMessage> {
        let len = self.messages.len();
        if len <= n {
            self.messages.iter().collect()
        } else {
            self.messages.iter().skip(len - n).collect()
        }
    }
}

/// 会话管理器
#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, ChatSession>,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
    
    /// 创建新会话
    pub fn create_session(&mut self, model_id: &str) -> Result<String> {
        let session = ChatSession::new(model_id);
        let session_id = session.id.clone();
        
        self.sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
    
    /// 删除会话
    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        self.sessions.remove(session_id)
            .context("Session not found")?;
        Ok(())
    }
    
    /// 获取会话
    pub fn get_session(&self, session_id: &str) -> Option<&ChatSession> {
        self.sessions.get(session_id)
    }
    
    /// 获取可变会话引用
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut ChatSession> {
        self.sessions.get_mut(session_id)
    }
    
    /// 列出所有会话
    pub fn list_sessions(&self) -> Result<Vec<&ChatSession>> {
        Ok(self.sessions.values().collect())
    }
    
    /// 清除会话历史
    pub fn clear_history(&mut self, session_id: &str) -> Result<()> {
        let session = self.sessions.get_mut(session_id)
            .context("Session not found")?;
        
        session.clear_history();
        Ok(())
    }
    
    /// 检查会话是否存在
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }
    
    /// 添加消息到会话
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let session = self.sessions.get_mut(session_id)
            .context("Session not found")?;
        
        if role == "user" {
            session.add_user_message(content);
        } else if role == "assistant" {
            session.add_assistant_message(content);
        } else {
            return Err(anyhow::anyhow!("Invalid role: {}", role));
        }
        
        Ok(())
    }
    
    /// ✅ 新增：获取 sessions HashMap 的可变引用（用于 state_machine）
    pub fn get_sessions_mut(&mut self) -> &mut HashMap<String, ChatSession> {
        &mut self.sessions
    }
    
    /// ✅ 新增：获取 sessions HashMap 的不可变引用
    pub fn get_sessions(&self) -> &HashMap<String, ChatSession> {
        &self.sessions
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
