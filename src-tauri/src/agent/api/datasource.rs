// src-tauri/src/agent/api/datasource.rs
//
// 数据源管理 API - 对标 Python 的 api/datasource.py
//
// 提供：
// - 上传文件（Excel/CSV）
// - 连接数据库
// - 数据源配置管理

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub filename: String,
    pub rows: usize,
    pub columns: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub source_type: String,
    pub connection_string: Option<String>,
    pub file_path: Option<String>,
    pub sheet_name: Option<String>,
}

/// 上传数据文件
#[tauri::command]
pub async fn upload_file(
    file_data: Vec<u8>,
    filename: String,
) -> Result<UploadResponse, String> {
    let extension = PathBuf::from(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !["xlsx", "xls", "csv"].contains(&extension.as_str()) {
        return Err("仅支持 .xlsx / .xls / .csv 文件".to_string());
    }

    // 解析文件并加载数据
    let df = match extension.as_str() {
        "csv" => {
            CsvReader::new(std::io::Cursor::new(&file_data))
                .finish()
                .map_err(|e| e.to_string())?
        }
        _ => {
            return Err("Excel 文件暂不支持，请使用 CSV 格式".to_string());
        }
    };

    // 更新全局数据
    let rows = df.height();
    let columns = df.width();

    crate::state::GLOBAL_DF.lock().map_err(|e| e.to_string())?
        .replace(df);

    Ok(UploadResponse {
        success: true,
        filename,
        rows,
        columns,
    })
}

/// 获取数据源列表
#[tauri::command]
pub fn list_datasources() -> Vec<DataSourceConfig> {
    let mut sources = Vec::new();

    if let Ok(df_guard) = crate::state::GLOBAL_DF.lock() {
        if df_guard.is_some() {
            sources.push(DataSourceConfig {
                source_type: "dataframe".to_string(),
                connection_string: None,
                file_path: None,
                sheet_name: None,
            });
        }
    }

    sources
}