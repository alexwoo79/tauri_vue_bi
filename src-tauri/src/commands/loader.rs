// src-tauri/src/commands/loader.rs
//
// 文件加载命令（File Loading Command）
//
// 负责 CSV / Excel 文件读取、自动类型推断、业务浮点转换以及 Tauri 命令注册。

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook_auto, Data as XlDataType, Reader};
use polars::prelude::*;
use regex::Regex;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::df_util::{df_to_payload, PREVIEW_LIMIT};
use crate::state::{
    persist_dataset_registry, register_dataset, ACTIVE_DATASET_ID, CLEAN_HISTORY, GLOBAL_DF,
    ORIGINAL_DF,
};
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Public output type
// ─────────────────────────────────────────────────────────────────────────────

pub struct LoadFileOutput {
    pub df: DataFrame,
    pub notices: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

/// Load a CSV or Excel file into the global DataFrame.
#[tauri::command]
pub async fn load_file(
    path: String,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
    header_locked: bool,
) -> ApiResult<ChartPayload> {
    match load_file_impl(&path, skip_head, skip_tail, header_row, header_locked) {
        Ok(output) => {
            let df = output.df;
            let payload = df_to_payload(&df, Some(PREVIEW_LIMIT));
            *GLOBAL_DF.lock().unwrap() = Some(df.clone());
            *ORIGINAL_DF.lock().unwrap() = Some(df.clone());
            CLEAN_HISTORY.lock().unwrap().clear();
            let file_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("加载数据")
                .to_string();
            register_dataset(&df, file_name, "load_file".to_string());
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

/// Load multiple CSV/Excel files, concatenate them, then set as global DataFrame.
#[tauri::command]
pub async fn load_files(
    paths: Vec<String>,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
    header_locked: bool,
    diagonal: Option<bool>,
) -> ApiResult<ChartPayload> {
    if paths.is_empty() {
        return ApiResult::failure("请至少选择一个文件");
    }

    let use_diagonal = diagonal.unwrap_or(true);

    let mut frames: Vec<DataFrame> = Vec::new();
    let mut notices: Vec<String> = Vec::new();

    for p in &paths {
        match load_file_impl(p, skip_head, skip_tail, header_row, header_locked) {
            Ok(output) => {
                frames.push(output.df);
                if !output.notices.is_empty() {
                    let file_name = std::path::Path::new(p)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(p.as_str())
                        .to_string();
                    for n in output.notices {
                        notices.push(format!("[{file_name}] {n}"));
                    }
                }
            }
            Err(e) => {
                return ApiResult::failure(format!("文件加载失败: {p}，原因: {e}"));
            }
        }
    }

    let merged = match concat_dataframes(frames, use_diagonal) {
        Ok(df) => df,
        Err(e) => return ApiResult::failure(e.to_string()),
    };

    let payload = df_to_payload(&merged, Some(PREVIEW_LIMIT));
    *GLOBAL_DF.lock().unwrap() = Some(merged.clone());
    *ORIGINAL_DF.lock().unwrap() = Some(merged.clone());
    CLEAN_HISTORY.lock().unwrap().clear();

    let dataset_name = if paths.len() == 1 {
        std::path::Path::new(&paths[0])
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("加载数据")
            .to_string()
    } else {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("批量加载_{}_文件_{ts}", paths.len())
    };
    register_dataset(&merged, dataset_name, "load_files".to_string());

    match payload {
        Ok(mut p) => {
            p.notices = notices;
            ApiResult::success(p)
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Load multiple files/folders as separate datasets, and preview the first file.
#[tauri::command]
pub async fn load_paths_as_datasets(
    paths: Vec<String>,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
    header_locked: bool,
) -> ApiResult<ChartPayload> {
    if paths.is_empty() {
        return ApiResult::failure("请至少选择一个文件或文件夹");
    }

    let files = match collect_supported_files(&paths) {
        Ok(list) => list,
        Err(e) => return ApiResult::failure(e.to_string()),
    };
    if files.is_empty() {
        return ApiResult::failure("未找到可加载的数据文件（支持 CSV / Excel）");
    }

    let mut first_df: Option<DataFrame> = None;
    let mut first_notices: Vec<String> = Vec::new();
    let mut first_dataset_id = String::new();

    for f in &files {
        let p = f.to_string_lossy().to_string();
        let output = match load_file_impl(&p, skip_head, skip_tail, header_row, header_locked) {
            Ok(v) => v,
            Err(e) => {
                return ApiResult::failure(format!("文件加载失败: {}，原因: {e}", f.display()))
            }
        };

        let file_name = f
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("加载数据")
            .to_string();
        let meta = register_dataset(&output.df, file_name, "load_paths_as_datasets".to_string());

        if first_df.is_none() {
            first_dataset_id = meta.id;
            first_notices = output.notices;
            first_df = Some(output.df);
        }
    }

    let Some(preview_df) = first_df else {
        return ApiResult::failure("未加载到可预览的数据");
    };

    *GLOBAL_DF.lock().unwrap() = Some(preview_df.clone());
    *ORIGINAL_DF.lock().unwrap() = Some(preview_df.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    *ACTIVE_DATASET_ID.lock().unwrap() = Some(first_dataset_id);
    if let Err(e) = persist_dataset_registry() {
        eprintln!("persist dataset registry failed: {e}");
    }

    match df_to_payload(&preview_df, Some(PREVIEW_LIMIT)) {
        Ok(mut p) => {
            p.notices = first_notices;
            ApiResult::success(p)
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

/// Return a summary of the currently loaded DataFrame.
#[tauri::command]
pub async fn get_dataframe_info(limit: Option<usize>) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return ApiResult::failure("No data loaded. Please select a file and click Load.")
            }
            Some(df) => df.clone(),
        }
    };
    let n = limit.unwrap_or(PREVIEW_LIMIT);
    match df_to_payload(&df, Some(n)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core implementation
// ─────────────────────────────────────────────────────────────────────────────

pub fn load_file_impl(
    path: &str,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
    header_locked: bool,
) -> Result<LoadFileOutput> {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let df = match ext.as_str() {
        "csv" => {
            if header_row >= 0 {
                let hr = header_row as usize;
                if header_locked {
                    let raw = read_csv_impl(p, false, 0)?;
                    promote_header_and_trim(raw, hr, skip_head, skip_tail)?
                } else {
                    let raw = read_csv_impl(p, false, skip_head)?;
                    promote_header_and_trim(raw, hr, 0, skip_tail)?
                }
            } else {
                let mut parsed = read_csv_impl(p, true, skip_head)?;
                if skip_tail > 0 {
                    let keep = parsed.height().saturating_sub(skip_tail);
                    parsed = parsed.slice(0, keep);
                }
                parsed
            }
        }
        "xlsx" | "xls" | "xlsm" | "ods" => {
            // ✅ 读取所有 sheet
            let all_sheets = if header_row >= 0 {
                let hr = header_row as usize;
                if header_locked {
                    let combined_skip = hr.saturating_add(skip_head);
                    read_all_excel_sheets(p, combined_skip, skip_tail)?
                } else {
                    let combined_skip = skip_head.saturating_add(hr);
                    read_all_excel_sheets(p, combined_skip, skip_tail)?
                }
            } else {
                read_all_excel_sheets(p, skip_head, skip_tail)?
            };

            // ✅ 第一个 sheet 作为主数据集
            let first_sheet = all_sheets.first()
                .ok_or_else(|| anyhow!("No valid sheets found"))?;
            
            let mut df = first_sheet.1.clone();
            
            // ✅ 将所有 sheet 注册为独立数据集
            for (sheet_name, sheet_df) in &all_sheets {
                // 清理和类型转换
                let cleaned_df = drop_all_null_cols(sheet_df.clone());
                let (cleaned_df, _notices) = auto_cast_business_float_columns(cleaned_df)?;
                let cleaned_df = auto_cast_temporal_columns(cleaned_df)?;
                
                // 注册数据集（使用 sheet 名称）
                crate::state::register_dataset(&cleaned_df, sheet_name.clone(), "excel_sheet".to_string());
            }
            
            df
        }
        other => bail!("Unsupported file extension: .{other}"),
    };

    let df = drop_all_null_cols(df);
    let (df, notices) = auto_cast_business_float_columns(df)?;
    let df = auto_cast_temporal_columns(df)?;
    Ok(LoadFileOutput { df, notices })
}

fn concat_dataframes(frames: Vec<DataFrame>, diagonal: bool) -> Result<DataFrame> {
    if frames.is_empty() {
        bail!("没有可拼接的数据");
    }
    if frames.len() == 1 {
        return Ok(frames.into_iter().next().unwrap());
    }

    let lazy_frames: Vec<LazyFrame> = frames.into_iter().map(|df| df.lazy()).collect();
    let args = if diagonal {
        UnionArgs {
            diagonal: true,
            ..Default::default()
        }
    } else {
        UnionArgs::default()
    };
    concat(lazy_frames, args)?.collect().map_err(Into::into)
}

fn collect_supported_files(paths: &[String]) -> Result<Vec<PathBuf>> {
    let mut out: BTreeSet<PathBuf> = BTreeSet::new();

    for raw in paths {
        if raw.trim().is_empty() {
            continue;
        }
        let p = Path::new(raw);
        if !p.exists() {
            bail!("路径不存在: {raw}");
        }

        if p.is_file() {
            if is_supported_file(p) {
                out.insert(p.to_path_buf());
            }
            continue;
        }

        if p.is_dir() {
            collect_dir_files(p, &mut out)?;
            continue;
        }
    }

    Ok(out.into_iter().collect())
}

fn collect_dir_files(dir: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_files(&path, out)?;
        } else if path.is_file() && is_supported_file(&path) {
            out.insert(path);
        }
    }
    Ok(())
}

fn is_supported_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "csv" | "xlsx" | "xls" | "xlsm" | "ods")
}

// ─────────────────────────────────────────────────────────────────────────────
// Business float auto-conversion
// ─────────────────────────────────────────────────────────────────────────────

fn auto_cast_business_float_columns(df: DataFrame) -> Result<(DataFrame, Vec<String>)> {
    let mut notices: Vec<String> = Vec::new();
    let mut out_cols: Vec<Column> = Vec::with_capacity(df.width());

    for col in df.get_columns() {
        let name = col.name().as_str().to_string();
        if !is_business_float_suffix(&name) {
            out_cols.push(col.clone());
            continue;
        }

        if col.dtype() == &DataType::String {
            let utf8 = col
                .as_materialized_series()
                .str()
                .map_err(|e| anyhow!("Column '{name}' to string chunked failed: {e}"))?;

            let mut float_vals: Vec<Option<f64>> = Vec::with_capacity(utf8.len());
            let mut trimmed_vals: Vec<Option<String>> = Vec::with_capacity(utf8.len());
            let mut failed_count = 0usize;
            let mut failed_samples: VecDeque<String> = VecDeque::new();

            for opt in utf8 {
                match opt {
                    None => {
                        float_vals.push(None);
                        trimmed_vals.push(None);
                    }
                    Some(raw) => {
                        let t = raw.trim();
                        if t.is_empty() {
                            float_vals.push(None);
                            trimmed_vals.push(None);
                            continue;
                        }

                        trimmed_vals.push(Some(t.to_string()));
                        if let Some(v) = parse_business_float_text(t) {
                            float_vals.push(Some(v));
                        } else {
                            float_vals.push(None);
                            failed_count += 1;
                            if failed_samples.len() < 3 {
                                failed_samples.push_back(t.to_string());
                            }
                        }
                    }
                }
            }

            if failed_count == 0 {
                out_cols.push(Series::new(name.as_str().into(), float_vals).into());
            } else {
                out_cols.push(Series::new(name.as_str().into(), trimmed_vals).into());
                let sample_text = failed_samples
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("、");
                notices.push(format!(
                    "列\"{name}\"包含 {failed_count} 个无法转换为浮点数的值（示例：{sample_text}），已保留为文本，请手工处理。"
                ));
            }
            continue;
        }

        if col.dtype().is_primitive_numeric() {
            match col.cast(&DataType::Float64) {
                Ok(casted) => out_cols.push(casted),
                Err(_) => {
                    out_cols.push(col.clone());
                    notices.push(format!(
                        "列\"{name}\"是数值列，但自动转换为 Float64 失败，请手工检查。"
                    ));
                }
            }
            continue;
        }

        out_cols.push(col.clone());
        notices.push(format!(
            "列\"{name}\"命中金额/数量等后缀，但当前类型为 {}，无法自动转换，请手工处理。",
            col.dtype()
        ));
    }

    let out_df = DataFrame::new(out_cols).map_err(|e| anyhow!("rebuild DataFrame failed: {e}"))?;
    Ok((out_df, notices))
}

fn is_business_float_suffix(name: &str) -> bool {
    const SUFFIXES: [&str; 7] = ["金额", "数量", "额", "款", "占比", "利润", "比率"];
    let n = name.trim_end();
    SUFFIXES.iter().any(|s| n.ends_with(s))
}

fn parse_business_float_text(raw: &str) -> Option<f64> {
    let mut s = raw.trim().replace([',', '，'], "");
    s = s.replace(['￥', '¥', '$', '元'], "");

    if let Some(stripped) = s.strip_suffix('%') {
        let v = stripped.trim().parse::<f64>().ok()?;
        return Some(v / 100.0);
    }

    s.parse::<f64>().ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV reader
// ─────────────────────────────────────────────────────────────────────────────

fn read_csv_impl(path: &Path, has_header: bool, skip_rows: usize) -> Result<DataFrame> {
    CsvReadOptions::default()
        .with_has_header(has_header)
        .with_skip_rows(skip_rows)
        .try_into_reader_with_file_path(Some(path.to_path_buf()))
        .context("Failed to open CSV")?
        .finish()
        .context("Failed to parse CSV")
}

fn promote_header_and_trim(
    mut df: DataFrame,
    header_row: usize,
    skip_head_after_header: usize,
    skip_tail: usize,
) -> Result<DataFrame> {
    if df.width() == 0 {
        return Ok(df);
    }
    if header_row >= df.height() {
        bail!(
            "Header row index {header_row} out of range (rows: {})",
            df.height()
        );
    }

    let new_names = sanitize_header_names(
        df.get_row(header_row)
            .map_err(|e| anyhow!("{e}"))?
            .0
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let raw = format!("{v}");
                let trimmed = raw.trim().trim_matches('"').trim().to_string();
                if trimmed.is_empty() {
                    format!("column_{i}")
                } else {
                    trimmed
                }
            })
            .collect(),
    );

    let old_names: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|n| n.as_str().to_string())
        .collect();

    for (i, old_name) in old_names.iter().enumerate() {
        df.rename(old_name, new_names[i].as_str().into())
            .map_err(|e| anyhow!("{e}"))?;
    }

    let data_start = header_row
        .saturating_add(1)
        .saturating_add(skip_head_after_header);
    if data_start >= df.height() {
        return Ok(df.slice(0, 0));
    }

    let available = df.height() - data_start;
    let keep = available.saturating_sub(skip_tail);
    Ok(df.slice(data_start as i64, keep))
}

fn sanitize_header_names(names: Vec<String>) -> Vec<String> {
    let mut used: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::with_capacity(names.len());

    for (i, name) in names.into_iter().enumerate() {
        let base = {
            let t = name.trim();
            if t.is_empty() {
                format!("column_{i}")
            } else {
                t.to_string()
            }
        };

        let mut candidate = base.clone();
        let mut suffix = 2usize;
        while used.contains(&candidate) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        used.insert(candidate.clone());
        out.push(candidate);
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Temporal column auto-cast
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum TemporalKind {
    Date,
    DateTime,
}

fn auto_cast_temporal_columns(df: DataFrame) -> Result<DataFrame> {
    let mut candidates: Vec<(String, TemporalKind)> = Vec::new();

    for col in df.get_columns() {
        if col.dtype() != &DataType::String {
            continue;
        }
        if let Some(kind) = infer_temporal_kind(col) {
            candidates.push((col.name().to_string(), kind));
        }
    }

    if candidates.is_empty() {
        return Ok(df);
    }

    let mut lf = df.lazy();
    for (name, kind) in candidates {
        let expr = match kind {
            TemporalKind::Date => col(name.as_str())
                .str()
                .to_date(StrptimeOptions {
                    format: None,
                    strict: false,
                    exact: true,
                    cache: true,
                })
                .alias(name.as_str()),
            TemporalKind::DateTime => col(name.as_str())
                .str()
                .to_datetime(
                    Some(TimeUnit::Milliseconds),
                    None,
                    StrptimeOptions {
                        format: None,
                        strict: false,
                        exact: true,
                        cache: true,
                    },
                    lit("raise"),
                )
                .alias(name.as_str()),
        };
        lf = lf.with_column(expr);
    }

    lf.collect()
        .map_err(|e| anyhow!("temporal type cast failed: {e}"))
}

fn infer_temporal_kind(col: &Column) -> Option<TemporalKind> {
    let re_date = Regex::new(r#"^"?\d{4}[-/]\d{1,2}[-/]\d{1,2}"?$"#).ok()?;
    let re_datetime =
        Regex::new(r#"^"?\d{4}[-/]\d{1,2}[-/]\d{1,2}[ T]\d{1,2}:\d{2}(:\d{2})?(\.\d+)?"?$"#)
            .ok()?;

    let name = col.name().to_lowercase();
    let has_time_hint = [
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

    let ca = col.as_materialized_series().str().ok()?;
    let mut sampled = 0usize;
    let mut date_hits = 0usize;
    let mut datetime_hits = 0usize;

    for v in ca {
        let raw = match v {
            Some(s) => s.trim(),
            None => continue,
        };
        if raw.is_empty() {
            continue;
        }

        sampled += 1;
        if re_datetime.is_match(raw) {
            datetime_hits += 1;
        } else if re_date.is_match(raw) {
            date_hits += 1;
        }

        if sampled >= 50 {
            break;
        }
    }

    if sampled < 3 {
        return None;
    }

    let threshold = if has_time_hint { 0.45 } else { 0.75 };
    let datetime_ratio = datetime_hits as f64 / sampled as f64;
    let date_ratio = (date_hits + datetime_hits) as f64 / sampled as f64;

    if datetime_ratio >= threshold {
        Some(TemporalKind::DateTime)
    } else if date_ratio >= threshold {
        Some(TemporalKind::Date)
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Excel reader (calamine) - 支持多 Sheet，每个 sheet 作为独立数据集
// ─────────────────────────────────────────────────────────────────────────────

/// 读取 Excel 文件的所有 sheet，返回第一个 sheet 的 DataFrame
/// 其他 sheet 会通过 side effect 注册到 DATASET_REGISTRY
fn read_excel_impl(path: &Path, skip_head: usize, skip_tail: usize) -> Result<DataFrame> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Cannot open Excel file: {}", path.display()))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        bail!("Excel file has no sheets");
    }

    // ✅ 对齐 Python 版本：遍历所有 sheet，每个 sheet 作为独立的 DataFrame
    let mut first_df: Option<DataFrame> = None;
    let mut sheet_count = 0usize;

    for sheet_name in &sheet_names {
        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Cannot read sheet '{}'", sheet_name))?;

        let total = range.height();
        if total == 0 {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no data)", sheet_name);
            continue;
        }

        let data_start = skip_head.min(total);
        let data_end = if skip_tail < total - data_start {
            total - skip_tail
        } else {
            data_start
        };

        if data_start >= data_end {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no rows after skip)", sheet_name);
            continue;
        }

        let header_row_idx = data_start;
        let col_count = range.width();

        let headers: Vec<String> = (0..col_count)
            .map(|c| {
                range
                    .get((header_row_idx, c))
                    .map(|cell| {
                        let s = cell_to_string(cell);
                        if s.trim().is_empty() {
                            format!("column_{c}")
                        } else {
                            s
                        }
                    })
                    .unwrap_or_else(|| format!("column_{c}"))
            })
            .collect();

        let data_rows: Vec<Vec<_>> = ((header_row_idx + 1)..data_end)
            .map(|r| (0..col_count).map(|c| range.get((r, c))).collect())
            .collect();

        if data_rows.is_empty() {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no data rows)", sheet_name);
            continue;
        }

        let mut series_vec: Vec<Column> = Vec::with_capacity(col_count);
        for c in 0..col_count {
            let name: PlSmallStr = headers[c].as_str().into();
            let all_numeric = data_rows.iter().all(|row| match row[c] {
                None | Some(XlDataType::Empty) => true,
                Some(XlDataType::Float(_)) | Some(XlDataType::Int(_)) | Some(XlDataType::Bool(_)) => {
                    true
                }
                _ => false,
            });

            let col: Column = if all_numeric {
                let vals: Vec<Option<f64>> = data_rows.iter().map(|row| cell_to_f64(row[c])).collect();
                Series::new(name, vals).into()
            } else {
                let vals: Vec<Option<String>> = data_rows
                    .iter()
                    .map(|row| {
                        let s = cell_to_string_opt(row[c]);
                        s.filter(|v| !v.is_empty())
                    })
                    .collect();
                Series::new(name, vals).into()
            };
            series_vec.push(col);
        }

        let df = DataFrame::new(series_vec).map_err(|e| anyhow!("{e}"))?;
        
        // ✅ 第一个 sheet 作为主数据集返回
        if first_df.is_none() {
            first_df = Some(df.clone());
        }
        
        // ✅ 将所有 sheet 注册为独立数据集（供后续切换使用）
        // 注意：这里不直接调用 register_dataset，因为该函数需要访问全局状态
        // 而是在 load_file_impl 中处理
        sheet_count += 1;
        tracing::info!("[ExcelDS] sheet '{}' loaded OK ({} rows)", sheet_name, df.height());
    }

    if sheet_count == 0 {
        bail!("Excel 文件中未发现有效工作表。");
    }

    // ✅ 返回第一个 sheet 的 DataFrame（其他 sheet 会在上层注册）
    first_df.ok_or_else(|| anyhow!("No valid sheets found"))
}

/// 读取 Excel 文件的所有 sheet，返回所有 DataFrame 及其对应的 sheet 名称
fn read_all_excel_sheets(
    path: &Path,
    skip_head: usize,
    skip_tail: usize,
) -> Result<Vec<(String, DataFrame)>> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Cannot open Excel file: {}", path.display()))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        bail!("Excel file has no sheets");
    }

    let mut results: Vec<(String, DataFrame)> = Vec::new();

    for sheet_name in &sheet_names {
        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Cannot read sheet '{}'", sheet_name))?;

        let total = range.height();
        if total == 0 {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no data)", sheet_name);
            continue;
        }

        let data_start = skip_head.min(total);
        let data_end = if skip_tail < total - data_start {
            total - skip_tail
        } else {
            data_start
        };

        if data_start >= data_end {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no rows after skip)", sheet_name);
            continue;
        }

        let header_row_idx = data_start;
        let col_count = range.width();

        let headers: Vec<String> = (0..col_count)
            .map(|c| {
                range
                    .get((header_row_idx, c))
                    .map(|cell| {
                        let s = cell_to_string(cell);
                        if s.trim().is_empty() {
                            format!("column_{c}")
                        } else {
                            s
                        }
                    })
                    .unwrap_or_else(|| format!("column_{c}"))
            })
            .collect();

        let data_rows: Vec<Vec<_>> = ((header_row_idx + 1)..data_end)
            .map(|r| (0..col_count).map(|c| range.get((r, c))).collect())
            .collect();

        if data_rows.is_empty() {
            tracing::warn!("[ExcelDS] sheet '{}' skipped (no data rows)", sheet_name);
            continue;
        }

        let mut series_vec: Vec<Column> = Vec::with_capacity(col_count);
        for c in 0..col_count {
            let name: PlSmallStr = headers[c].as_str().into();
            let all_numeric = data_rows.iter().all(|row| match row[c] {
                None | Some(XlDataType::Empty) => true,
                Some(XlDataType::Float(_)) | Some(XlDataType::Int(_)) | Some(XlDataType::Bool(_)) => {
                    true
                }
                _ => false,
            });

            let col: Column = if all_numeric {
                let vals: Vec<Option<f64>> = data_rows.iter().map(|row| cell_to_f64(row[c])).collect();
                Series::new(name, vals).into()
            } else {
                let vals: Vec<Option<String>> = data_rows
                    .iter()
                    .map(|row| {
                        let s = cell_to_string_opt(row[c]);
                        s.filter(|v| !v.is_empty())
                    })
                    .collect();
                Series::new(name, vals).into()
            };
            series_vec.push(col);
        }

        let df = DataFrame::new(series_vec).map_err(|e| anyhow!("{e}"))?;
        let df_height = df.height(); // ✅ 先获取高度
        results.push((sheet_name.clone(), df));
        tracing::info!("[ExcelDS] sheet '{}' loaded OK ({} rows)", sheet_name, df_height);
    }

    if results.is_empty() {
        bail!("Excel 文件中未发现有效工作表。");
    }

    Ok(results)
}

fn cell_to_string(cell: &XlDataType) -> String {
    match cell {
        XlDataType::Empty => String::new(),
        XlDataType::String(s) => s.clone(),
        XlDataType::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        XlDataType::Int(i) => i.to_string(),
        XlDataType::Bool(b) => b.to_string(),
        XlDataType::Error(e) => format!("{e:?}"),
        XlDataType::DateTime(dt) => dt
            .as_datetime()
            .map(|value| {
                if value.time().to_string() == "00:00:00" {
                    value.date().format("%Y-%m-%d").to_string()
                } else {
                    value.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            })
            .unwrap_or_else(|| format!("{dt}")),
        XlDataType::DateTimeIso(s) => s.clone(),
        XlDataType::DurationIso(s) => s.clone(),
    }
}

fn cell_to_string_opt(cell: Option<&XlDataType>) -> Option<String> {
    match cell {
        None | Some(XlDataType::Empty) => None,
        Some(c) => Some(cell_to_string(c)),
    }
}

fn cell_to_f64(cell: Option<&XlDataType>) -> Option<f64> {
    match cell {
        None | Some(XlDataType::Empty) => None,
        Some(XlDataType::Float(f)) => Some(*f),
        Some(XlDataType::Int(i)) => Some(*i as f64),
        Some(XlDataType::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn drop_all_null_cols(df: DataFrame) -> DataFrame {
    let keep: Vec<PlSmallStr> = df
        .get_columns()
        .iter()
        .filter(|s| s.null_count() < s.len())
        .map(|s| s.name().clone())
        .collect();
    df.select(keep).unwrap_or(df)
}
