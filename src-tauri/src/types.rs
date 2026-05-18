// src-tauri/src/types.rs
//
// 共享数据类型（Shared Data Types）
//
// 所有跨模块共用的数据结构定义集中在此文件。
// 前端通过 JSON 序列化接收这些类型，字段命名遵循 camelCase ↔ snake_case 自动转换。

use serde::{Deserialize, Serialize};

/// Generic API response wrapper sent back to the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResult<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn failure(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// Schema info for a single column.
#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub dtype: String,
    pub nullable: bool,
}

/// Row data as a map of column name -> value.
pub type RowMap = serde_json::Map<String, serde_json::Value>;

/// Payload sent to frontend for chart rendering.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChartPayload {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<RowMap>,
    pub total_rows: usize,
    pub notices: Vec<String>,
}

/// Dataset metadata.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetMeta {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at_ms: u64,
    pub created_at_ms: u64,
}

/// Dataset record for persistence.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub created_at: u64,
}

/// Runtime dataset entry (contains both meta and DataFrame)
#[derive(Debug, Clone)]
pub struct RuntimeDataset {
    pub meta: DatasetMeta,
    pub df: DataFrame,
}

use polars::prelude::DataFrame;

/// Persisted dataset state.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedDatasetState {
    pub datasets: Vec<DatasetRecord>,
    pub current_id: Option<String>,
}
