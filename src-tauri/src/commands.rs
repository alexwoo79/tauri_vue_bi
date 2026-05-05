// src-tauri/src/commands.rs
//
// 后端核心逻辑 (Backend Core Logic)
//
// All heavy data-processing logic lives here so that `lib.rs` stays a clean
// Tauri wiring layer.  Each `*_impl` function is a pure Rust function that
// receives a reference to a Polars `DataFrame` and returns a transformed
// `DataFrame` (or an error).
//
// Sections
// ────────
//   1. File loading     – CSV / Excel via Polars
//   2. Chart data       – column selection, sorting, TopN filtering
//   3. Pivot table      – multi-dimensional aggregation via Polars pivot
//   4. Data cleaning    – column filter, row filter, fillna, dedup, trim, find/replace, type-cast
//   5. GroupBy          – groupby + aggregation
//
// All functions return `anyhow::Result<DataFrame>` for ergonomic `?` chaining.

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook_auto, Data as XlDataType, Reader};
use polars::prelude::*;
use polars_ops::frame::pivot::{pivot, PivotAgg};
use regex::Regex;
use rust_xlsxwriter::{Format, Workbook};
use std::borrow::Cow;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
enum TemporalKind {
    Date,
    DateTime,
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. File loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load a CSV or Excel file from `path`.
///
/// - Rows `[0, skip_head)` are dropped from the top.
/// - Rows `[len - skip_tail, len)` are dropped from the bottom.
/// - If `header_row >= 0`, that (0-based, post-skip) row is promoted to header.
pub fn load_file_impl(
    path: &str,
    skip_head: usize,
    skip_tail: usize,
    header_row: i64,
) -> Result<DataFrame> {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut df = match ext.as_str() {
        "csv" => {
            // Read without a header so we can honour skip/header_row ourselves.
            CsvReadOptions::default()
                .with_has_header(true) // Polars requires has_header=true to use skip_rows, even if we rename later
                .with_skip_rows(skip_head)
                .try_into_reader_with_file_path(Some(p.to_path_buf()))
                .context("Failed to open CSV")?
                .finish()
                .context("Failed to parse CSV")?
        }
        "xlsx" | "xls" | "xlsm" | "ods" => {
            read_excel_impl(p, skip_head, skip_tail)?
        }
        other => bail!("Unsupported file extension: .{other}"),
    };

    // Drop trailing rows
    if skip_tail > 0 && skip_tail < df.height() {
        df = df.slice(0, df.height() - skip_tail);
    }

    // Promote header row
    if header_row >= 0 {
        let hr = header_row as usize;
        if hr < df.height() {
            // Extract the header row values as strings
            let new_names: Vec<String> = df
                .get_row(hr)
                .map_err(|e| anyhow!("{e}"))?
                .0
                .iter()
                .map(|v| format!("{v}"))
                .collect();
            // Drop the header row from data
            df = df.slice(hr as i64 + 1, df.height() - hr - 1);
            // Rename columns
            for (i, name) in new_names.into_iter().enumerate() {
                df.rename(&format!("column_{i}"), name.as_str().into())
                    .map_err(|e| anyhow!("{e}"))?;
            }
        }
    }

    // Drop fully-null rows and columns
    let df = drop_all_null_cols(df);

    // Auto-cast likely temporal string columns to real Polars Date/Datetime.
    // This improves downstream default field inference for Gantt and date filters.
    let df = auto_cast_temporal_columns(df)?;
    Ok(df)
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
                .to_date(
                    StrptimeOptions {
                        format: None,
                        strict: false,
                        exact: true,
                        cache: true,
                    },
                )
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

    lf.collect().map_err(|e| anyhow!("temporal type cast failed: {e}"))
}

fn infer_temporal_kind(col: &Column) -> Option<TemporalKind> {
    let re_date = Regex::new(r#"^"?\d{4}[-/]\d{1,2}[-/]\d{1,2}"?$"#).ok()?;
    let re_datetime = Regex::new(r#"^"?\d{4}[-/]\d{1,2}[-/]\d{1,2}[ T]\d{1,2}:\d{2}(:\d{2})?(\.\d+)?"?$"#).ok()?;

    let name = col.name().to_lowercase();
    let has_time_hint = [
        "date", "time", "start", "end", "begin", "finish", "deadline", "due", "milestone", "created", "updated",
        "日期", "时间", "开始", "结束", "里程碑", "截止",
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
// Excel reader (calamine)
// ─────────────────────────────────────────────────────────────────────────────

/// Read the first sheet of an Excel file into a DataFrame.
/// Uses the first non-skipped row as the header.
fn read_excel_impl(path: &Path, skip_head: usize, skip_tail: usize) -> Result<DataFrame> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Cannot open Excel file: {}", path.display()))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names
        .first()
        .ok_or_else(|| anyhow!("Excel file has no sheets"))?;

    let range = workbook
        .worksheet_range(sheet_name)
        .with_context(|| format!("Cannot read sheet '{}'", sheet_name))?;

    let total = range.height();
    if total == 0 {
        bail!("Sheet '{}' is empty", sheet_name);
    }

    // Determine usable row range
    let data_start = skip_head.min(total);
    let data_end = if skip_tail < total - data_start {
        total - skip_tail
    } else {
        data_start
    };

    if data_start >= data_end {
        bail!("No rows remaining after skipping head/tail");
    }

    // First row in the range becomes the header
    let header_row_idx = data_start;
    let col_count = range.width();

    // Build header names from the first data row
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

    // Collect data rows (skip header row)
    let data_rows: Vec<Vec<_>> = ((header_row_idx + 1)..data_end)
        .map(|r| (0..col_count).map(|c| range.get((r, c))).collect())
        .collect();

    if data_rows.is_empty() {
        // Return empty DataFrame with correct schema
        let cols: Vec<Column> = headers
            .iter()
            .map(|h| Column::new(h.as_str().into(), Vec::<String>::new()))
            .collect();
        return DataFrame::new(cols).map_err(|e| anyhow!("{e}"));
    }

    // Build one Series per column, auto-detecting type
    let mut series_vec: Vec<Column> = Vec::with_capacity(col_count);
    for c in 0..col_count {
        let name: PlSmallStr = headers[c].as_str().into();
        // Check if all non-null values are numeric
        let all_numeric = data_rows.iter().all(|row| {
            match row[c] {
                None | Some(XlDataType::Empty) => true,
                Some(XlDataType::Float(_))
                | Some(XlDataType::Int(_))
                | Some(XlDataType::Bool(_)) => true,
                _ => false,
            }
        });

        let col: Column = if all_numeric {
            let vals: Vec<Option<f64>> = data_rows
                .iter()
                .map(|row| cell_to_f64(row[c]))
                .collect();
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

    DataFrame::new(series_vec).map_err(|e| anyhow!("{e}"))
}

fn cell_to_string(cell: &XlDataType) -> String {
    match cell {
        XlDataType::Empty => String::new(),
        XlDataType::String(s) => s.clone(),
        XlDataType::Float(f) => {
            // Show integers without decimal if whole number
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

// ─────────────────────────────────────────────────────────────────────────────
// File saver (CSV + Excel)
// ─────────────────────────────────────────────────────────────────────────────

/// Save a DataFrame to `path`. Format is inferred from the file extension.
/// Supported: `.csv`, `.xlsx`
pub fn save_file_impl(df: &DataFrame, path: &str) -> Result<()> {
    let p = Path::new(path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" => save_csv(df, p),
        "xlsx" => save_xlsx(df, p),
        other => bail!("Unsupported save format: .{other} (use .csv or .xlsx)"),
    }
}

fn save_csv(df: &DataFrame, path: &Path) -> Result<()> {
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create file: {}", path.display()))?;
    CsvWriter::new(&mut file)
        .finish(&mut df.clone())
        .map_err(|e| anyhow!("CSV write error: {e}"))
}

fn save_xlsx(df: &DataFrame, path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    // Header style
    let header_fmt = Format::new().set_bold();

    // Write headers
    let columns = df.get_columns();
    for (c, col) in columns.iter().enumerate() {
        sheet
            .write_string_with_format(0, c as u16, col.name().as_str(), &header_fmt)
            .map_err(|e| anyhow!("xlsx header write error: {e}"))?;
    }

    // Write data rows
    for r in 0..df.height() {
        for (c, col) in columns.iter().enumerate() {
            let row = (r + 1) as u32;
            let col_idx = c as u16;
            let series = col.as_materialized_series();

            match series.get(r).unwrap_or(AnyValue::Null) {
                AnyValue::Null => {} // leave cell empty
                AnyValue::Boolean(v) => {
                    sheet
                        .write_boolean(row, col_idx, v)
                        .map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int8(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int16(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int32(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Int64(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt8(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt16(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt32(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::UInt64(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Float32(v) => {
                    sheet.write_number(row, col_idx, v as f64).map_err(|e| anyhow!("{e}"))?;
                }
                AnyValue::Float64(v) => {
                    sheet.write_number(row, col_idx, v).map_err(|e| anyhow!("{e}"))?;
                }
                other => {
                    sheet
                        .write_string(row, col_idx, &format!("{other}"))
                        .map_err(|e| anyhow!("{e}"))?;
                }
            }
        }
    }

    workbook
        .save(path)
        .map_err(|e| anyhow!("xlsx save error: {e}"))
}

/// Drop columns that are entirely null.
fn drop_all_null_cols(df: DataFrame) -> DataFrame {
    let keep: Vec<PlSmallStr> = df
        .get_columns()
        .iter()
        .filter(|s| s.null_count() < s.len())
        .map(|s| s.name().clone())
        .collect();
    df.select(keep).unwrap_or(df)
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Chart data
// ─────────────────────────────────────────────────────────────────────────────

/// Prepare DataFrame for chart rendering.
///
/// Pipeline: sort → topN
#[allow(clippy::too_many_arguments)]
pub fn fetch_chart_data_impl(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    color_col: Option<&str>,
    sort_by: &str,    // "x" | "y" | "none"
    sort_asc: bool,
    top_n: i64,       // 0 = no limit, >0 = top-N, <0 = bottom-N
) -> Result<DataFrame> {
    let mut keep = vec![x_col, y_col];
    if let Some(c) = color_col {
        if !keep.contains(&c) {
            keep.push(c);
        }
    }

    let mut result = df.select(keep).map_err(|e| anyhow!("{e}"))?;

    // Sort
    let sort_col = match sort_by {
        "x" => Some(x_col),
        "y" => Some(y_col),
        _ => None,
    };
    if let Some(col) = sort_col {
        result = result
            .sort(
                [col],
                SortMultipleOptions::default().with_order_descending(!sort_asc),
            )
            .map_err(|e| anyhow!("{e}"))?;
    }

    // TopN
    if top_n > 0 {
        let n = (top_n as usize).min(result.height());
        result = result.slice(0, n);
    } else if top_n < 0 {
        let n = ((-top_n) as usize).min(result.height());
        let start = result.height().saturating_sub(n);
        result = result.slice(start as i64, n);
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Pivot table
// ─────────────────────────────────────────────────────────────────────────────

/// Build a pivot table using Polars.
pub fn pivot_data_impl(
    df: &DataFrame,
    rows: &[String],
    columns: &[String],
    values: &[String],
    agg: &str,
) -> Result<DataFrame> {
    if rows.is_empty() || values.is_empty() {
        bail!("rows and values must not be empty");
    }

    let agg_fn = match agg {
        "sum" => PivotAgg::Sum,
        "mean" => PivotAgg::Mean,
        "count" => PivotAgg::Count,
        "min" => PivotAgg::Min,
        "max" => PivotAgg::Max,
        other => bail!("Unknown aggregation function: {other}"),
    };

    if columns.is_empty() {
        let agg_expr = match agg {
            "sum" => col(values[0].as_str()).sum(),
            "mean" => col(values[0].as_str()).mean(),
            "count" => col(values[0].as_str()).count(),
            "min" => col(values[0].as_str()).min(),
            "max" => col(values[0].as_str()).max(),
            _ => unreachable!(),
        }
        .alias(values[0].as_str());

        let result = df
            .clone()
            .lazy()
            .group_by(rows.iter().map(|c| col(c.as_str())).collect::<Vec<_>>())
            .agg([agg_expr])
            .collect()
            .map_err(|e| anyhow!("{e}"))?;

        return Ok(result);
    }

    let result = pivot(
        df,
        columns.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
        Some(rows.iter().map(|c| c.as_str()).collect::<Vec<_>>()),
        Some(values.iter().map(|c| c.as_str()).collect::<Vec<_>>()),
        false,
        Some(agg_fn),
        None,
    )
    .map_err(|e| anyhow!("{e}"))?;

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Data cleaning
// ─────────────────────────────────────────────────────────────────────────────

/// Apply a sequential pipeline of cleaning operations to `df`.
#[allow(clippy::too_many_arguments)]
pub fn clean_data_impl(
    df: &DataFrame,
    filter_cols: &[String],
    row_filter_col: &str,
    row_filter_op: &str,
    row_filter_val: &str,
    fillna_col: &str,
    fillna_val: &str,
    dedup_cols: &[String],
    trim_cols: &[String],
    fr_cols: &[String],
    find_text: &str,
    replace_text: &str,
    use_regex: bool,
    type_col: &str,
    type_target: &str,
) -> Result<DataFrame> {
    // 1. Column removal (empty = keep all)
    let mut work_df = if filter_cols.is_empty() {
        df.clone()
    } else {
        let all_names = df.get_column_names();
        let drop_cols: Vec<&str> = filter_cols
            .iter()
            .filter(|c| all_names.iter().any(|n| n.as_str() == c.as_str()))
            .map(|c| c.as_str())
            .collect();
        if drop_cols.is_empty() {
            df.clone()
        } else {
            df.drop_many(drop_cols)
        }
    };

    // 2. Row condition filter (optional)
    if !row_filter_col.is_empty() {
        if work_df.column(row_filter_col).is_err() {
            bail!("Row filter column not found: {row_filter_col}");
        }
        let mask = build_row_filter_mask(&work_df, row_filter_col, row_filter_op, row_filter_val)?;
        work_df = work_df
            .filter(&mask)
            .map_err(|e| anyhow!("row filter failed: {e}"))?;
    }

    let mut lf: LazyFrame = work_df.lazy();

    // 3. Fill nulls in a single column.
    // If the same column is also type-cast in this request, cast it to String
    // first so fill-null uses a compatible literal and avoids runtime panics.
    if !fillna_col.is_empty() {
        let fill_expr = lit(fillna_val.to_string());
        let fill_col_expr = if fillna_col == type_col && !type_target.is_empty() {
            col(fillna_col)
                .cast(DataType::String)
                .fill_null(fill_expr)
                .alias(fillna_col)
        } else {
            col(fillna_col)
                .fill_null(fill_expr)
                .alias(fillna_col)
        };
        lf = lf.with_column(fill_col_expr);
    }

    // 4. Deduplicate
    let subset: Option<Vec<String>> = if dedup_cols.is_empty() {
        None
    } else {
        Some(dedup_cols.to_vec())
    };
    lf = lf.unique(subset, UniqueKeepStrategy::First);

    // Collect early so we can do mutable string operations
    let mut df2 = lf.collect().map_err(|e| anyhow!("{e}"))?;

    // 5. Trim whitespace from selected string columns
    for c in trim_cols {
        if let Ok(column) = df2.column(c) {
            let series = column.as_materialized_series();
            if series.dtype() == &DataType::String {
                let trimmed = series
                    .str()
                    .map_err(|e| anyhow!("{e}"))?
                    .apply(|opt| opt.map(|s| Cow::Owned(s.trim().to_string())))
                    .into_series()
                    .with_name(c.as_str().into());
                df2.replace(c, trimmed).map_err(|e| anyhow!("{e}"))?;
            }
        }
    }

    // 6. Find & replace
    if !find_text.is_empty() {
        let regex = if use_regex {
            Some(Regex::new(find_text).map_err(|e| anyhow!("Invalid regex: {e}"))?)
        } else {
            None
        };

        for c in fr_cols {
            if let Ok(column) = df2.column(c) {
                let ca = column.cast(&DataType::String).map_err(|e| anyhow!("{e}"))?;
                let str_ca = ca.str().map_err(|e| anyhow!("{e}"))?;
                let replaced = str_ca
                    .apply(|opt| {
                        opt.map(|s| {
                            if let Some(re) = &regex {
                                Cow::Owned(re.replace_all(s, replace_text).into_owned())
                            } else {
                                Cow::Owned(s.replace(find_text, replace_text))
                            }
                        })
                    })
                    .into_series()
                    .with_name(c.as_str().into());

                df2.replace(c, replaced)
                    .map_err(|e| anyhow!("{e}"))?;
            }
        }
    }

    // 7. Type-cast a column
    if !type_col.is_empty() {
        let src_dtype = df2
            .column(type_col)
            .map_err(|e| anyhow!("{e}"))?
            .dtype()
            .clone();

        // String → Date / Datetime must use str().to_date/to_datetime (Polars lazy API).
        // Numeric → Date / Datetime can use direct cast.
        let is_string_src = matches!(src_dtype, DataType::String);

        df2 = match type_target {
            "datetime" if is_string_src => {
                df2.lazy()
                    .with_column(
                        col(type_col)
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
                            .alias(type_col),
                    )
                    .collect()
                    .map_err(|e| anyhow!("datetime parse failed: {e}"))?
            }
            "date" if is_string_src => {
                df2.lazy()
                    .with_column(
                        col(type_col)
                            .str()
                            .to_date(StrptimeOptions {
                                format: None,
                                strict: false,
                                exact: true,
                                cache: true,
                            })
                            .alias(type_col),
                    )
                    .collect()
                    .map_err(|e| anyhow!("date parse failed: {e}"))?
            }
            _ => {
                // All other casts (int, float, str, or numeric→date/datetime)
                let target_dtype = match type_target {
                    "int" => DataType::Int64,
                    "float" => DataType::Float64,
                    "str" => DataType::String,
                    "datetime" => DataType::Datetime(TimeUnit::Milliseconds, None),
                    "date" => DataType::Date,
                    other => bail!("Unknown target type: {other}"),
                };
                let series = df2
                    .column(type_col)
                    .map_err(|e| anyhow!("{e}"))?
                    .as_materialized_series()
                    .cast(&target_dtype)
                    .map_err(|e| anyhow!("cast failed: {e}"))?;
                df2.replace(type_col, series.with_name(type_col.into()))
                    .map_err(|e| anyhow!("{e}"))?;
                df2
            }
        };
    }

    Ok(df2)
}

fn build_row_filter_mask(
    df: &DataFrame,
    row_filter_col: &str,
    row_filter_op: &str,
    row_filter_val: &str,
) -> Result<BooleanChunked> {
    let series = df
        .column(row_filter_col)
        .map_err(|e| anyhow!("row filter column read failed: {e}"))?
        .as_materialized_series();

    let needs_value = !matches!(row_filter_op, "is_null" | "not_null");
    if needs_value && row_filter_val.is_empty() {
        bail!("Row filter value is required for operator: {row_filter_op}");
    }

    let value_num = row_filter_val.parse::<f64>().ok();

    let mask_values: Vec<bool> = (0..series.len())
        .map(|idx| {
            let av = series.get(idx).unwrap_or(AnyValue::Null);
            match row_filter_op {
                "is_null" => matches!(av, AnyValue::Null),
                "not_null" => !matches!(av, AnyValue::Null),
                "eq" => any_value_to_string(&av).is_some_and(|s| s == row_filter_val),
                "ne" => any_value_to_string(&av).is_some_and(|s| s != row_filter_val),
                "gt" => compare_numeric(&av, value_num, |l, r| l > r),
                "ge" => compare_numeric(&av, value_num, |l, r| l >= r),
                "lt" => compare_numeric(&av, value_num, |l, r| l < r),
                "le" => compare_numeric(&av, value_num, |l, r| l <= r),
                "contains" => any_value_to_string(&av).is_some_and(|s| s.contains(row_filter_val)),
                "not_contains" => any_value_to_string(&av).is_some_and(|s| !s.contains(row_filter_val)),
                "starts_with" => any_value_to_string(&av).is_some_and(|s| s.starts_with(row_filter_val)),
                "ends_with" => any_value_to_string(&av).is_some_and(|s| s.ends_with(row_filter_val)),
                other => {
                    let _ = other;
                    false
                }
            }
        })
        .collect();

    if !matches!(
        row_filter_op,
        "eq"
            | "ne"
            | "gt"
            | "ge"
            | "lt"
            | "le"
            | "contains"
            | "not_contains"
            | "starts_with"
            | "ends_with"
            | "is_null"
            | "not_null"
    ) {
        bail!("Unknown row filter operator: {row_filter_op}");
    }

    Ok(BooleanChunked::from_slice("mask".into(), &mask_values))
}

fn compare_numeric(av: &AnyValue<'_>, rhs: Option<f64>, pred: fn(f64, f64) -> bool) -> bool {
    let Some(r) = rhs else {
        return false;
    };
    let Some(l) = any_value_to_f64(av) else {
        return false;
    };
    pred(l, r)
}

fn any_value_to_f64(av: &AnyValue<'_>) -> Option<f64> {
    match av {
        AnyValue::Int8(v) => Some(*v as f64),
        AnyValue::Int16(v) => Some(*v as f64),
        AnyValue::Int32(v) => Some(*v as f64),
        AnyValue::Int64(v) => Some(*v as f64),
        AnyValue::UInt8(v) => Some(*v as f64),
        AnyValue::UInt16(v) => Some(*v as f64),
        AnyValue::UInt32(v) => Some(*v as f64),
        AnyValue::UInt64(v) => Some(*v as f64),
        AnyValue::Float32(v) => Some(*v as f64),
        AnyValue::Float64(v) => Some(*v),
        AnyValue::String(s) => s.parse::<f64>().ok(),
        AnyValue::StringOwned(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn any_value_to_string(av: &AnyValue<'_>) -> Option<String> {
    match av {
        AnyValue::Null => None,
        AnyValue::String(v) => Some((*v).to_string()),
        AnyValue::StringOwned(v) => Some(v.to_string()),
        _ => Some(format!("{av}")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. GroupBy aggregation
// ─────────────────────────────────────────────────────────────────────────────

pub fn groupby_agg_impl(
    df: &DataFrame,
    group_cols: &[String],
    agg_col: &str,
    agg_func: &str,
) -> Result<DataFrame> {
    if group_cols.is_empty() {
        bail!("group_cols must not be empty");
    }

    let agg_expr = match agg_func {
        "sum" => col(agg_col).sum(),
        "mean" => col(agg_col).mean(),
        "count" => col(agg_col).count(),
        "min" => col(agg_col).min(),
        "max" => col(agg_col).max(),
        other => bail!("Unknown aggregation function: {other}"),
    }
    .alias(agg_col);

    let result = df
        .clone()
        .lazy()
        .group_by(group_cols.iter().map(|c| col(c.as_str())).collect::<Vec<_>>())
        .agg([agg_expr])
        .sort(
            group_cols.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            SortMultipleOptions::default(),
        )
        .collect()
        .map_err(|e| anyhow!("{e}"))?;

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// NOTE: Excel reading
//
// Full Excel (.xlsx / .xls / .xlsm) reading is not yet enabled in this build.
// The `xlsx2csv` Polars feature converts sheets to CSV in-memory, but the
// exact stable API differs across Polars minor versions.
//
// To add real Excel support, integrate the `calamine` crate:
//   1. Add `calamine = "0.25"` to Cargo.toml.
//   2. Read the workbook with `calamine::open_workbook::<Xlsx, _>(path)`.
//   3. Convert each row to a Polars Series and build a DataFrame.
//
// Until then, the `load_file_impl` function returns a descriptive error for
// Excel files, asking the user to convert to CSV first.
// ─────────────────────────────────────────────────────────────────────────────
