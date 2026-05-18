// src-tauri/src/agent/api/system.rs
//
// 系统 API - 对标 Python 的 api/system.py
//
// 提供：
// - 系统更新检查
// - 系统状态
// - 健康检查

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub rust_version: String,
    pub tauri_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub download_url: Option<String>,
}

/// 获取系统状态
#[tauri::command]
pub fn get_system_status() -> SystemStatus {
    let rust_version = match option_env!("RUSTC_VERSION") {
        Some(v) => v.to_string(),
        None => "unknown".to_string(),
    };
    
    SystemStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        rust_version,
        tauri_version: tauri::VERSION.to_string(),
    }
}

/// 健康检查
#[tauri::command]
pub fn health_check() -> Result<bool, String> {
    // 检查全局数据状态
    let df_guard = crate::state::GLOBAL_DF.lock().map_err(|e| e.to_string())?;
    let _ = df_guard.as_ref();
    
    Ok(true)
}

/// 获取更新信息
#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    
    // TODO: 实现从 GitHub 检查最新版本
    // 目前返回当前版本信息
    Ok(UpdateInfo {
        current_version,
        latest_version: None,
        update_available: false,
        download_url: None,
    })
}

/// 获取应用日志目录
#[tauri::command]
pub fn get_log_dir() -> Result<String, String> {
    // 返回日志目录路径
    #[cfg(target_os = "windows")]
    {
        Ok(std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| ".".to_string())
            + "\\tauri-vue-bi\\logs")
    }
    #[cfg(target_os = "macos")]
    {
        Ok(std::env::var("HOME")
            .unwrap_or_else(|_| ".".to_string())
            + "/Library/Logs/tauri-vue-bi")
    }
    #[cfg(target_os = "linux")]
    {
        Ok(std::env::var("HOME")
            .unwrap_or_else(|_| ".".to_string())
            + "/.local/share/tauri-vue-bi/logs")
    }
}