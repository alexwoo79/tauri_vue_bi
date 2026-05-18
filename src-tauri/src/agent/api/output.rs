// src-tauri/src/agent/api/output.rs
//
// 输出/下载 API - 对标 Python 的 api/output.py
//
// 提供：
// - 下载导出文件（Excel、Word等）

use std::path::PathBuf;

/// 导出目录路径
fn get_export_dir() -> PathBuf {
    PathBuf::from("outputs/exports")
}

/// 检查文件是否存在并返回文件路径
#[tauri::command]
pub fn get_export_path(filename: String) -> Result<String, String> {
    // 安全检查：防止路径遍历
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }
    
    let export_dir = get_export_dir();
    let file_path = export_dir.join(&filename);
    
    if !file_path.exists() {
        return Err(format!("File not found: {}", filename));
    }
    
    Ok(file_path.to_string_lossy().to_string())
}

/// 获取导出目录中的所有文件
#[tauri::command]
pub fn list_exports() -> Result<Vec<String>, String> {
    let export_dir = get_export_dir();
    
    if !export_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&export_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
        }
    }
    
    Ok(files)
}

/// 删除导出文件
#[tauri::command]
pub fn delete_export(filename: String) -> Result<bool, String> {
    // 安全检查：防止路径遍历
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err("Invalid filename".to_string());
    }
    
    let export_dir = get_export_dir();
    let file_path = export_dir.join(&filename);
    
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Err("File not found".to_string())
    }
}