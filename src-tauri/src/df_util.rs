// src-tauri/src/df_util.rs
//
// DataFrame → ChartPayload 工具函数（DataFrame Utility Functions）
//
// 将 Polars DataFrame 序列化为前端可消费的 ChartPayload JSON 结构。

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use once_cell::sync::Lazy;
use polars::prelude::*;
use polars::series::Series;
use regex::Regex;
use serde_json;

use crate::types::{ChartPayload, ColumnInfo, RowMap};

pub const PREVIEW_LIMIT: usize = 200;
pub const CHART_LIMIT: usize = 5_000;

pub fn df_to_payload(df: &DataFrame, limit: Option<usize>) -> Result<ChartPayload> {
    let total_rows = df.height();
    let preview_n = limit.map(|l| l.min(total_rows)).unwrap_or(total_rows);

    let columns: Vec<ColumnInfo> = df
        .get_columns()
        .iter()
        .map(|s| ColumnInfo {
            name: s.name().to_string(),
            dtype: s.dtype().to_string(),
            nullable: s.null_count() > 0,
        })
        .collect();

    let mut rows: Vec<RowMap> = Vec::new();
    let n_rows = df.height().min(preview_n);
    let column_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    
    for row_idx in 0..n_rows {
        let mut row_map = RowMap::new();
        for col_name in &column_names {
            let json_val = match df.column(col_name.as_str()) {
                Ok(col) => {
                    match col.get(row_idx) {
                        Ok(v) => v_to_json(&v),
                        Err(_) => serde_json::Value::Null,
                    }
                }
                Err(_) => serde_json::Value::Null,
            };
            row_map.insert(col_name.clone(), json_val);
        }
        rows.push(row_map);
    }

    Ok(ChartPayload {
        columns,
        rows,
        total_rows,
        notices: Vec::new(),
    })
}

fn v_to_json(v: &AnyValue) -> serde_json::Value {
    match v {
        AnyValue::Null => serde_json::Value::Null,
        AnyValue::Boolean(b) => serde_json::Value::Bool(*b),
        AnyValue::Int8(i) => serde_json::Value::Number((*i).into()),
        AnyValue::Int16(i) => serde_json::Value::Number((*i).into()),
        AnyValue::Int32(i) => serde_json::Value::Number((*i).into()),
        AnyValue::Int64(i) => serde_json::Value::Number((*i).into()),
        AnyValue::UInt8(i) => serde_json::Value::Number((*i).into()),
        AnyValue::UInt16(i) => serde_json::Value::Number((*i).into()),
        AnyValue::UInt32(i) => serde_json::Value::Number((*i).into()),
        AnyValue::UInt64(i) => serde_json::Value::Number((*i).into()),
        AnyValue::Float32(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f as f64).unwrap()),
        AnyValue::Float64(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap()),
        AnyValue::String(s) => serde_json::Value::String(s.to_string()),
        AnyValue::Binary(b) => serde_json::Value::String(STANDARD.encode::<&[u8]>(b)),
        AnyValue::Date(d) => serde_json::Value::String(d.to_string()),
        AnyValue::Datetime(d, _, _) => serde_json::Value::String(d.to_string()),
        AnyValue::Time(t) => serde_json::Value::String(t.to_string()),
        AnyValue::Duration(d, _) => serde_json::Value::Number((*d).into()),
        AnyValue::List(_) => serde_json::Value::Array(Vec::new()),
        AnyValue::Struct(_, _, _) => serde_json::Value::Object(RowMap::new()),
        _ => serde_json::Value::Null,
    }
}

pub fn infer_column_type(s: &Series) -> String {
    let dtype = s.dtype();
    
    if dtype.is_integer() || dtype.is_float() {
        "numeric".to_string()
    } else if dtype.is_temporal() {
        "datetime".to_string()
    } else if dtype.is_string() {
        "string".to_string()
    } else if dtype.is_bool(){
        "boolean".to_string()
    } else {
        "other".to_string()
    }
}

pub fn detect_date_column(s: &Series) -> bool {
    if !s.dtype().is_string() {
        return false;
    }
    
    let sample_size = s.len().min(100);
    let mut date_count = 0;
    
    static DATE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
        vec![
            Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap(),
            Regex::new(r"^\d{2}/\d{2}/\d{4}$").unwrap(),
            Regex::new(r"^\d{2}/\d{2}/\d{2}$").unwrap(),
            Regex::new(r"^\d{4}/\d{2}/\d{2}$").unwrap(),
            Regex::new(r"^\d{1,2}\s+[A-Za-z]+\s+\d{4}$").unwrap(),
        ]
    });
    
    for i in 0..sample_size {
        if let Ok(v) = s.get(i) {
            if let AnyValue::String(s) = v {
                if DATE_PATTERNS.iter().any(|p| p.is_match(s)) {
                    date_count += 1;
                }
            }
        }
    }
    
    date_count > sample_size / 2
}

pub fn detect_date_column_col(col: &Column) -> bool {
    if !col.dtype().is_string() {
        return false;
    }
    
    let sample_size = col.len().min(100);
    let mut date_count = 0;
    
    static DATE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
        vec![
            Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap(),
            Regex::new(r"^\d{2}/\d{2}/\d{4}$").unwrap(),
            Regex::new(r"^\d{2}/\d{2}/\d{2}$").unwrap(),
            Regex::new(r"^\d{4}/\d{2}/\d{2}$").unwrap(),
            Regex::new(r"^\d{1,2}\s+[A-Za-z]+\s+\d{4}$").unwrap(),
        ]
    });
    
    for i in 0..sample_size {
        if let Ok(v) = col.get(i) {
            if let AnyValue::String(s) = v {
                if DATE_PATTERNS.iter().any(|p| p.is_match(s)) {
                    date_count += 1;
                }
            }
        }
    }
    
    date_count > sample_size / 2
}

pub fn get_columns_by_type(df: &DataFrame, col_type: &str) -> Vec<String> {
    let col_names: Vec<String> = df.get_column_names().iter().map(|s| s.to_string()).collect();
    col_names
        .iter()
        .filter(|name| {
            match df.column(name.as_str()) {
                Ok(col) => {
                    let dtype = col.dtype();
                    match col_type {
                        "numeric" => dtype.is_integer() || dtype.is_float(),
                        "string" => dtype.is_string(),
                        "datetime" => dtype.is_temporal() || detect_date_column_col(col),
                        "boolean" => dtype.is_bool(),
                        _ => true,
                    }
                }
                Err(_) => false,
            }
        })
        .cloned()
        .collect()
}
