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

/// A lightweight, serialisable representation of a DataFrame column.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub dtype: String,
}

/// Represents a single row as a map of column-name → JSON value,
/// compatible with ECharts `dataset.source`.
pub type RowMap = serde_json::Map<String, serde_json::Value>;

/// Payload returned by `fetch_chart_data` and similar commands.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChartPayload {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<RowMap>,
    pub total_rows: usize,
    pub notices: Vec<String>,
}

/// Lightweight metadata for a registered dataset (serialised to frontend).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetMeta {
    pub id: String,
    pub name: String,
    pub source: String,
    pub total_rows: usize,
    pub total_cols: usize,
    pub created_at_ms: u128,
}

/// Full in-memory dataset record (DataFrame + metadata).
#[derive(Debug, Clone)]
pub struct DatasetRecord {
    pub meta: DatasetMeta,
    pub df: polars::prelude::DataFrame,
}

/// Persisted state written to / read from `state.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedDatasetState {
    pub active_dataset_id: Option<String>,
    pub datasets: Vec<DatasetMeta>,
}
