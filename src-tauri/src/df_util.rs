// src-tauri/src/df_util.rs
//
// DataFrame → ChartPayload 工具函数（DataFrame Utility Functions）
//
// 将 Polars DataFrame 序列化为前端可消费的 ChartPayload JSON 结构。
// 同时提供列类型推断辅助函数，用于让前端在不了解 Polars 内部类型的情况下
// 正确渲染日期/时间类字段。

use anyhow::Result;
use once_cell::sync::Lazy;
use polars::prelude::*;
use regex::Regex;

use crate::types::{ChartPayload, ColumnInfo, RowMap};

/// Maximum rows serialised when returning a preview (load / clean results).
pub const PREVIEW_LIMIT: usize = 200;
/// Maximum rows serialised for chart / pivot / groupby results.
pub const CHART_LIMIT: usize = 5_000;

/// Convert a Polars DataFrame to a `ChartPayload`.
///
/// `limit` caps the number of rows serialised (preview mode). Pass `None` to
/// serialise all rows.
pub fn df_to_payload(df: &DataFrame, limit: Option<usize>) -> Result<ChartPayload> {
    let total_rows = df.height();
    let preview_n = limit.map(|l| l.min(total_rows)).unwrap_or(total_rows);

    let columns: Vec<ColumnInfo> = df
        .get_columns()
        .iter()
        .map(|s| ColumnInfo {
            name: s.name().to_string(),
            dtype: infer_payload_dtype(s),
        })
        .collect();

    let col_names: Vec<String> = df
        .get_columns()
        .iter()
        .map(|s| s.name().to_string())
        .collect();

    let mut rows: Vec<RowMap> = Vec::with_capacity(preview_n);
    for row_idx in 0..preview_n {
        let mut map = serde_json::Map::with_capacity(col_names.len());
        for (i, column) in df.get_columns().iter().enumerate() {
            let val = series_value_to_json(column, row_idx);
            map.insert(col_names[i].clone(), val);
        }
        rows.push(map);
    }

    Ok(ChartPayload {
        columns,
        rows,
        total_rows,
        notices: Vec::new(),
    })
}

fn infer_payload_dtype(s: &Column) -> String {
    let real = format!("{}", s.dtype());
    if s.dtype() != &DataType::String {
        return real;
    }
    if let Some(inferred) = infer_temporal_string_dtype(s) {
        return inferred;
    }
    real
}

fn infer_temporal_string_dtype(s: &Column) -> Option<String> {
    static DATE_RE_YYYY_MM_DD: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}$").expect("valid date regex"));
    static DATE_RE_DD_MM_YYYY: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d{1,2}[-/]\d{1,2}[-/]\d{4}$").expect("valid date regex"));
    static DATE_RE_YYYYMMDD: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d{8}$").expect("valid date regex"));
    static DATETIME_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}[ T]\d{1,2}:\d{2}(:\d{2})?(\.\d+)?$")
            .expect("valid datetime regex")
    });

    let name = s.name().to_lowercase();
    let has_time_name_hint = [
        "date",
        "time",
        "start",
        "end",
        "begin",
        "finish",
        "deadline",
        "due",
        "milestone",
        "created",
        "updated",
        "日期",
        "时间",
        "开始",
        "结束",
        "里程碑",
        "截止",
    ]
    .iter()
    .any(|k| name.contains(k));

    let utf8 = s.as_materialized_series().str().ok()?;
    let mut sampled = 0usize;
    let mut date_hits = 0usize;
    let mut datetime_hits = 0usize;

    for opt in utf8 {
        let raw = match opt {
            Some(v) => v.trim(),
            None => continue,
        };
        if raw.is_empty() {
            continue;
        }
        sampled += 1;

        if DATETIME_RE.is_match(raw)
            || (raw.contains(':')
                && (DATE_RE_YYYY_MM_DD.is_match(raw.split(' ').next().unwrap_or(""))
                    || raw.contains('T')))
        {
            datetime_hits += 1;
        } else if DATE_RE_YYYY_MM_DD.is_match(raw)
            || DATE_RE_DD_MM_YYYY.is_match(raw)
            || DATE_RE_YYYYMMDD.is_match(raw)
        {
            date_hits += 1;
        }

        if sampled >= 50 {
            break;
        }
    }

    if sampled < 3 {
        return None;
    }

    let threshold = if has_time_name_hint { 0.45 } else { 0.75 };
    let datetime_ratio = datetime_hits as f64 / sampled as f64;
    let date_ratio = (date_hits + datetime_hits) as f64 / sampled as f64;

    if datetime_ratio >= threshold {
        Some("datetime".to_string())
    } else if date_ratio >= threshold {
        Some("date".to_string())
    } else {
        None
    }
}

/// Extract a single cell from a `Column` as a JSON `Value`.
pub fn series_value_to_json(s: &Column, idx: usize) -> serde_json::Value {
    use serde_json::Value;
    use AnyValue::*;

    match s
        .as_materialized_series()
        .get(idx)
        .unwrap_or(AnyValue::Null)
    {
        Null => Value::Null,
        Boolean(v) => Value::Bool(v),
        Int8(v) => Value::Number(v.into()),
        Int16(v) => Value::Number(v.into()),
        Int32(v) => Value::Number(v.into()),
        Int64(v) => Value::Number(v.into()),
        UInt8(v) => Value::Number(v.into()),
        UInt16(v) => Value::Number(v.into()),
        UInt32(v) => Value::Number(v.into()),
        UInt64(v) => Value::Number(v.into()),
        Float32(v) => serde_json::Number::from_f64(v as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Float64(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        other => Value::String(format!("{other}")),
    }
}
