// src-tauri/src/lib.rs
//
// 引擎入口 (Engine Entry Point)
//
// This module wires together:
//   • Tauri application bootstrap
//   • Global in-memory DataFrame state (via Polars LazyFrame + DuckDB)
//   • All Tauri commands exposed to the Vue3 frontend
//
// Architecture overview
// ─────────────────────
//   Frontend (Vue3/TS)  ──invoke──▶  Tauri IPC  ──▶  commands.rs
//                                                         │
//                                              Polars (load / clean / pivot / groupby)

pub mod commands;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Mutex;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use polars::io::parquet::read::ParquetReader;
use polars::io::parquet::write::ParquetWriter;
use polars::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Global shared state
// ─────────────────────────────────────────────────────────────────────────────

/// The currently loaded DataFrame, shared across all Tauri commands.
/// Wrapped in `Mutex` so that concurrent invocations are serialised.
pub static GLOBAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// Snapshot of the DataFrame right after `load_file`, used for rollback.
pub static ORIGINAL_DF: Lazy<Mutex<Option<DataFrame>>> = Lazy::new(|| Mutex::new(None));
/// History stack for step-wise clean undo.
pub static CLEAN_HISTORY: Lazy<Mutex<Vec<DataFrame>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// In-memory dataset registry for switching between loaded/derived datasets.
pub static DATASET_REGISTRY: Lazy<Mutex<Vec<DatasetRecord>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// Currently active dataset id.
pub static ACTIVE_DATASET_ID: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// ─────────────────────────────────────────────────────────────────────────────
// Shared data types (used by both lib.rs and commands.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Generic API response wrapper sent back to the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResult<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResult<T> {
    pub fn success(data: T) -> Self {
        Self { ok: true, data: Some(data), error: None }
    }
    pub fn failure(msg: impl Into<String>) -> Self {
        Self { ok: false, data: None, error: Some(msg.into()) }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetMeta {
    pub id: String,
    pub name: String,
    pub source: String,
    pub total_rows: usize,
    pub total_cols: usize,
    pub created_at_ms: u128,
}

#[derive(Debug, Clone)]
pub struct DatasetRecord {
    pub meta: DatasetMeta,
    pub df: DataFrame,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedDatasetState {
    active_dataset_id: Option<String>,
    datasets: Vec<DatasetMeta>,
}

fn dataset_store_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home)
        .join(".tauri_vue_bi")
        .join("dataset_registry")
}

fn dataset_state_file() -> PathBuf {
    dataset_store_dir().join("state.json")
}

fn dataset_parquet_file(dataset_id: &str) -> PathBuf {
    dataset_store_dir().join(format!("{dataset_id}.parquet"))
}

fn dataset_legacy_csv_file(dataset_id: &str) -> PathBuf {
    dataset_store_dir().join(format!("{dataset_id}.csv"))
}

fn persist_dataset_registry() -> Result<()> {
    let dir = dataset_store_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("create dataset store dir failed: {}", dir.display()))?;

    let active_id = ACTIVE_DATASET_ID.lock().unwrap().clone();
    let records: Vec<DatasetRecord> = DATASET_REGISTRY.lock().unwrap().clone();

    for rec in &records {
        let file_path = dataset_parquet_file(rec.meta.id.as_str());
        let mut f = std::fs::File::create(&file_path)
            .with_context(|| format!("create dataset file failed: {}", file_path.display()))?;
        ParquetWriter::new(&mut f)
            .finish(&mut rec.df.clone())
            .map_err(|e| anyhow::anyhow!("write dataset parquet failed: {e}"))?;
    }

    let keep_ids: HashSet<String> = records.iter().map(|r| r.meta.id.clone()).collect();
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("read dataset store dir failed: {}", dir.display()))?
    {
        let entry = entry.map_err(|e| anyhow::anyhow!("read_dir entry error: {e}"))?;
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("parquet") && ext != Some("csv") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if !keep_ids.contains(stem) {
            let _ = fs::remove_file(path);
        }
    }

    let state = PersistedDatasetState {
        active_dataset_id: active_id,
        datasets: records.into_iter().map(|r| r.meta).collect(),
    };
    let state_file = dataset_state_file();
    let state_text = serde_json::to_string_pretty(&state)?;
    fs::write(&state_file, state_text)
        .with_context(|| format!("write state file failed: {}", state_file.display()))?;

    Ok(())
}

fn load_persisted_dataset_registry() -> Result<()> {
    let state_file = dataset_state_file();
    if !state_file.exists() {
        return Ok(());
    }

    let state_text = fs::read_to_string(&state_file)
        .with_context(|| format!("read state file failed: {}", state_file.display()))?;
    let state: PersistedDatasetState = serde_json::from_str(&state_text)?;

    let mut records: Vec<DatasetRecord> = Vec::new();
    let mut migrated_from_csv = false;
    for mut meta in state.datasets {
        let parquet_file = dataset_parquet_file(meta.id.as_str());
        let legacy_csv_file = dataset_legacy_csv_file(meta.id.as_str());

        let df = if parquet_file.exists() {
            let mut f = std::fs::File::open(&parquet_file)
                .with_context(|| format!("open dataset parquet failed: {}", parquet_file.display()))?;
            ParquetReader::new(&mut f)
                .finish()
                .map_err(|e| anyhow::anyhow!("read dataset parquet failed: {e}"))?
        } else if legacy_csv_file.exists() {
            migrated_from_csv = true;
            CsvReadOptions::default()
                .with_has_header(true)
                .try_into_reader_with_file_path(Some(legacy_csv_file.clone()))
                .map_err(|e| anyhow::anyhow!("open legacy dataset csv failed: {e}"))?
                .finish()
                .map_err(|e| anyhow::anyhow!("read legacy dataset csv failed: {e}"))?
        } else {
            continue;
        };

        meta.total_rows = df.height();
        meta.total_cols = df.width();
        records.push(DatasetRecord { meta, df });
    }

    let restored_active_id = state.active_dataset_id.and_then(|id| {
        if records.iter().any(|r| r.meta.id == id) {
            Some(id)
        } else {
            None
        }
    });

    *DATASET_REGISTRY.lock().unwrap() = records.clone();
    *ACTIVE_DATASET_ID.lock().unwrap() = restored_active_id.clone();

    let current = if let Some(id) = restored_active_id {
        records.iter().find(|r| r.meta.id == id).cloned()
    } else {
        records.first().cloned()
    };

    if let Some(rec) = current {
        *GLOBAL_DF.lock().unwrap() = Some(rec.df.clone());
        *ORIGINAL_DF.lock().unwrap() = Some(rec.df.clone());
        *ACTIVE_DATASET_ID.lock().unwrap() = Some(rec.meta.id);
    }

    if migrated_from_csv {
        // Rewrite all restored datasets as parquet and clean stale legacy CSV files.
        persist_dataset_registry()?;
    }

    Ok(())
}

fn register_dataset(df: &DataFrame, name: String, source: String) -> DatasetMeta {
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let id = format!("ds_{created_at_ms}");
    let meta = DatasetMeta {
        id: id.clone(),
        name,
        source,
        total_rows: df.height(),
        total_cols: df.width(),
        created_at_ms,
    };

    {
        DATASET_REGISTRY.lock().unwrap().push(DatasetRecord {
            meta: meta.clone(),
            df: df.clone(),
        });
        *ACTIVE_DATASET_ID.lock().unwrap() = Some(id);
    }
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }
    meta
}

fn sync_active_dataset(df: &DataFrame) {
    let active_id = ACTIVE_DATASET_ID.lock().unwrap().clone();
    let Some(id) = active_id else { return; };

    let mut registry = DATASET_REGISTRY.lock().unwrap();
    if let Some(rec) = registry.iter_mut().find(|r| r.meta.id == id) {
        rec.df = df.clone();
        rec.meta.total_rows = df.height();
        rec.meta.total_cols = df.width();
    }
    drop(registry);
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: convert Polars DataFrame ─▶ ChartPayload
// ─────────────────────────────────────────────────────────────────────────────

pub fn df_to_payload(df: &DataFrame, limit: Option<usize>) -> Result<ChartPayload> {
    let total_rows = df.height();
    let preview_n = limit.map(|l| l.min(total_rows)).unwrap_or(total_rows);

    // Build column metadata
    let columns: Vec<ColumnInfo> = df
        .get_columns()
        .iter()
        .map(|s| ColumnInfo {
            name: s.name().to_string(),
            dtype: infer_payload_dtype(s),
        })
        .collect();

    // Pre-collect column names once to avoid per-row allocations (key optimization)
    let col_names: Vec<String> = df
        .get_columns()
        .iter()
        .map(|s| s.name().to_string())
        .collect();

    // Serialise only the preview rows to a JSON map
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
    static DATETIME_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d{4}[-/]\d{1,2}[-/]\d{1,2}[ T]\d{1,2}:\d{2}(:\d{2})?(\.\d+)?$").expect("valid datetime regex"));

    let name = s.name().to_lowercase();
    let has_time_name_hint = [
        "date", "time", "start", "end", "begin", "finish", "deadline", "due", "milestone", "created", "updated",
        "日期", "时间", "开始", "结束", "里程碑", "截止",
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

        if DATETIME_RE.is_match(raw) || (raw.contains(':') && (DATE_RE_YYYY_MM_DD.is_match(raw.split(' ').next().unwrap_or("")) || raw.contains('T'))) {
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
fn series_value_to_json(s: &Column, idx: usize) -> serde_json::Value {
    use serde_json::Value;
    use AnyValue::*;

    match s.as_materialized_series().get(idx).unwrap_or(AnyValue::Null) {
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
        Float32(v) => {
            serde_json::Number::from_f64(v as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        Float64(v) => {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        other => Value::String(format!("{other}")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core Tauri commands (high-level orchestration)
// ─────────────────────────────────────────────────────────────────────────────

/// 预览行数上限 — 加载/清洗时返回给前端的最大行数
const PREVIEW_LIMIT: usize = 200;
/// 图表渲染行数上限 — chart/pivot/groupby 结果的最大序列化行数
const CHART_LIMIT: usize = 5_000;

/// 从全局 DF 中取出一个 clone（立即释放 Mutex），若无数据返回 Err。
macro_rules! take_df {
    () => {{
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("No data loaded. Please select a file and click Load."),
            Some(df) => df.clone(),
        }
    }};
}

/// Load a CSV or Excel file into the global DataFrame.
///
/// Parameters
/// ──────────
/// `path`       – absolute path to the file (chosen via the frontend file dialog)
/// `skip_head`  – number of rows to skip at the top
/// `skip_tail`  – number of rows to skip at the bottom
/// `header_row` – 0-based index of the header row (-1 = first row is header)
/// `header_locked` – when true, lock header_row as absolute file row; skip_head applies below header
#[tauri::command]
async fn load_file(
    path: String,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
    header_locked: bool,
) -> ApiResult<ChartPayload> {
    match commands::load_file_impl(&path, skip_head, skip_tail, header_row, header_locked) {
        Ok(output) => {
            let df = output.df;
            // 只序列化预览行，全量 DataFrame 保存到全局状态
            let payload = df_to_payload(&df, Some(PREVIEW_LIMIT));
            *GLOBAL_DF.lock().unwrap() = Some(df.clone());
            *ORIGINAL_DF.lock().unwrap() = Some(df.clone());
            CLEAN_HISTORY.lock().unwrap().clear();
            let file_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("加载数据")
                .to_string();
            register_dataset(
                &df,
                file_name,
                "load_file".to_string(),
            );
            match payload {
                Ok(mut p) => {
                    p.notices = output.notices;
                    ApiResult::success(p)
                }
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Return a summary of the currently loaded DataFrame (columns + first N rows).
#[tauri::command]
async fn get_dataframe_info(limit: Option<usize>) -> ApiResult<ChartPayload> {
    let df = take_df!();
    let n = limit.unwrap_or(PREVIEW_LIMIT);
    match df_to_payload(&df, Some(n)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Fetch and transform data for chart rendering.
///
/// This is the primary command called by `ChartAnalysis.vue`.
///
/// Parameters
/// ──────────
/// `x_col`      – X-axis column name
/// `y_cols`     – one or more Y-axis column names (numeric recommended)
/// `color_col`  – optional grouping/colour column
/// `sort_by`    – "x" | "y" | "none"
/// `sort_asc`   – true = ascending
/// `top_n`      – 0 = no limit; > 0 = keep top N rows; < 0 = keep bottom N rows
#[tauri::command]
async fn fetch_chart_data(
    x_col: String,
    y_cols: Vec<String>,
    color_col: Option<String>,
    sort_by: String,
    sort_asc: bool,
    top_n: i64,
) -> ApiResult<ChartPayload> {
    let df = take_df!();
    match commands::fetch_chart_data_impl(
        &df,
        &x_col,
        &y_cols,
        color_col.as_deref(),
        &sort_by,
        sort_asc,
        top_n,
    ) {
        Ok(result_df) => match df_to_payload(&result_df, Some(CHART_LIMIT)) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Build a pivot table from the currently loaded DataFrame.
///
/// Parameters
/// ──────────
/// `rows`    – columns to use as row-index groups
/// `columns` – columns to use as column-index groups
/// `values`  – columns to aggregate
/// `agg`     – aggregation function: "sum" | "mean" | "count" | "min" | "max"
#[tauri::command]
async fn pivot_data(
    rows: Vec<String>,
    columns: Vec<String>,
    values: Vec<String>,
    agg: String,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = take_df!();
    match commands::pivot_data_impl(&df, &rows, &columns, &values, &agg) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let default_name = format!("透视结果_{}", values.join("_"));
                register_dataset(
                    &result_df,
                    dataset_name.unwrap_or(default_name),
                    "pivot_data".to_string(),
                );
            }

            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Apply cleaning operations to the global DataFrame and return the result.
///
/// The cleaning steps are applied in this order:
///   1. column-filter → 2. row-filter → 3. fillna → 4. dedup → 5. trim → 6. find/replace → 7. type-cast
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn clean_data(
    filter_cols: Vec<String>,
    row_filter_col: String,
    row_filter_op: String,
    row_filter_val: String,
    fillna_col: String,
    fillna_val: String,
    dedup_cols: Vec<String>,
    trim_cols: Vec<String>,
    fr_cols: Vec<String>,
    find_text: String,
    replace_text: String,
    use_regex: bool,
    type_cols: Vec<String>,
    type_target: String,
) -> ApiResult<ChartPayload> {
    let df = take_df!();
    CLEAN_HISTORY.lock().unwrap().push(df.clone());
    match commands::clean_data_impl(
        &df,
        &filter_cols,
        &row_filter_col,
        &row_filter_op,
        &row_filter_val,
        &fillna_col,
        &fillna_val,
        &dedup_cols,
        &trim_cols,
        &fr_cols,
        &find_text,
        &replace_text,
        use_regex,
        &type_cols,
        &type_target,
    ) {
        Ok(result_df) => {
            // Persist cleaned result so the next clean operation continues from
            // the latest state instead of reusing the original loaded DataFrame.
            *GLOBAL_DF.lock().unwrap() = Some(result_df.clone());
            sync_active_dataset(&result_df);
            match df_to_payload(&result_df, Some(PREVIEW_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => {
            // Revert push-on-entry when clean failed.
            CLEAN_HISTORY.lock().unwrap().pop();
            ApiResult::failure(e.to_string())
        }
    }
}

/// Undo the last cleaning step.
#[tauri::command]
async fn undo_clean() -> ApiResult<ChartPayload> {
    let prev = {
        let mut history = CLEAN_HISTORY.lock().unwrap();
        match history.pop() {
            None => return ApiResult::failure("No clean step to undo."),
            Some(df) => df,
        }
    };

    *GLOBAL_DF.lock().unwrap() = Some(prev.clone());
    sync_active_dataset(&prev);
    match df_to_payload(&prev, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Rollback all cleaning changes and restore the DataFrame to the initial
/// state captured at the latest `load_file` call.
#[tauri::command]
async fn rollback_clean() -> ApiResult<ChartPayload> {
    let original = {
        let guard = ORIGINAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("No original data snapshot. Please load a file first."),
            Some(df) => df.clone(),
        }
    };

    *GLOBAL_DF.lock().unwrap() = Some(original.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    sync_active_dataset(&original);
    match df_to_payload(&original, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// GroupBy aggregation: group by `group_cols`, aggregate `agg_col` with `agg_func`.
#[tauri::command]
async fn groupby_agg(
    group_cols: Vec<String>,
    agg_col: String,
    agg_func: String,
) -> ApiResult<ChartPayload> {
    let df = take_df!();
    match commands::groupby_agg_impl(&df, &group_cols, &agg_col, &agg_func) {
        Ok(result_df) => match df_to_payload(&result_df, Some(CHART_LIMIT)) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Fetch Gantt chart data: returns rows with task, start, end, and optional group/milestone columns.
#[tauri::command]
async fn fetch_gantt_data(
    task_col: String,
    start_col: String,
    end_col: String,
    project_col: Option<String>,
    color_col: Option<String>,
    milestone_col: Option<String>,
    detail_col: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = take_df!();
    let mut keep_cols: Vec<String> =
        vec![task_col.clone(), start_col.clone(), end_col.clone()];
    if let Some(ref c) = project_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = color_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = milestone_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = detail_col {
        keep_cols.push(c.clone());
    }
    // Keep only existing columns to avoid Polars error
    let column_names = df.get_column_names();
    let valid: Vec<&str> = keep_cols
        .iter()
        .filter(|c| column_names.iter().any(|n| n.as_str() == c.as_str()))
        .map(|c| c.as_str())
        .collect();

    match df.select(valid) {
        Ok(result_df) => match df_to_payload(&result_df, None) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Save the current DataFrame (or cleaned result) to a file.
/// `format_hint` is ignored — the extension in `path` determines the format.
/// Supports: `.csv`, `.xlsx`
#[tauri::command]
async fn save_file(path: String) -> ApiResult<String> {
    let df = take_df!();
    match commands::save_file_impl(&df, &path) {
        Ok(()) => ApiResult::success(path),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
async fn list_datasets() -> ApiResult<Vec<DatasetMeta>> {
    let mut metas: Vec<DatasetMeta> = DATASET_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.meta.clone())
        .collect();
    metas.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
    ApiResult::success(metas)
}

#[tauri::command]
async fn switch_dataset(dataset_id: String) -> ApiResult<ChartPayload> {
    let found = DATASET_REGISTRY
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.meta.id == dataset_id)
        .cloned();

    let Some(rec) = found else {
        return ApiResult::failure("Dataset not found");
    };

    *GLOBAL_DF.lock().unwrap() = Some(rec.df.clone());
    *ORIGINAL_DF.lock().unwrap() = Some(rec.df.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    *ACTIVE_DATASET_ID.lock().unwrap() = Some(rec.meta.id);
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }

    match df_to_payload(&rec.df, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
async fn save_current_dataset(name: String, source: Option<String>) -> ApiResult<DatasetMeta> {
    let df = take_df!();
    let meta = register_dataset(
        &df,
        if name.trim().is_empty() { "子数据集".to_string() } else { name.trim().to_string() },
        source.unwrap_or_else(|| "manual_save".to_string()),
    );
    ApiResult::success(meta)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri app bootstrap
// ─────────────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = load_persisted_dataset_registry() {
        eprintln!("load persisted dataset registry failed: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_file,
            get_dataframe_info,
            fetch_chart_data,
            pivot_data,
            clean_data,
            undo_clean,
            rollback_clean,
            groupby_agg,
            fetch_gantt_data,
            save_file,
            list_datasets,
            switch_dataset,
            save_current_dataset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
