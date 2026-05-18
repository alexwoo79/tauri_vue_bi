// src-tauri/src/agent/api/chat.rs
//
// 对话 API - 对标 Python 的 api/chat.py
//
// 提供：
// - 获取图表 HTML
// - 图表服务


/// 获取图表 HTML
#[tauri::command]
pub fn get_chart(chart_id: String) -> Result<String, String> {
    let store = crate::state::GLOBAL_CHART_STORE.lock().map_err(|e| e.to_string())?;

    store.get(&chart_id)
        .cloned()
        .ok_or_else(|| "Chart not found".to_string())
}

/// 列出所有图表
#[tauri::command]
pub fn list_charts() -> Vec<String> {
    let store = crate::state::GLOBAL_CHART_STORE.lock().unwrap();
    store.keys().cloned().collect()
}

/// 清除图表存储
#[tauri::command]
pub fn clear_charts() -> Result<bool, String> {
    let mut store = crate::state::GLOBAL_CHART_STORE.lock().map_err(|e| e.to_string())?;
    store.clear();
    Ok(true)
}