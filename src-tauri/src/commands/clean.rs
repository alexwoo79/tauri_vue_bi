// src-tauri/src/commands/clean.rs
//
// 数据清洗命令（Data Cleaning Command）

use anyhow::{anyhow, bail, Result};
use polars::prelude::*;
use regex::Regex;
use std::borrow::Cow;

use crate::df_util::{df_to_payload, PREVIEW_LIMIT};
use crate::state::{sync_active_dataset, CLEAN_HISTORY, GLOBAL_DF, ORIGINAL_DF};
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn clean_data(
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
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return ApiResult::failure(
                    "No data loaded. Please select a file and click Load.",
                )
            }
            Some(df) => df.clone(),
        }
    };
    CLEAN_HISTORY.lock().unwrap().push(df.clone());
    match clean_data_impl(
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
            *GLOBAL_DF.lock().unwrap() = Some(result_df.clone());
            sync_active_dataset();
            match df_to_payload(&result_df, Some(PREVIEW_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => {
            CLEAN_HISTORY.lock().unwrap().pop();
            ApiResult::failure(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn undo_clean() -> ApiResult<ChartPayload> {
    let prev = {
        let mut history = CLEAN_HISTORY.lock().unwrap();
        match history.pop() {
            None => return ApiResult::failure("No clean step to undo."),
            Some(df) => df,
        }
    };

    *GLOBAL_DF.lock().unwrap() = Some(prev.clone());
    sync_active_dataset();
    match df_to_payload(&prev, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

#[tauri::command]
pub async fn rollback_clean() -> ApiResult<ChartPayload> {
    let original = {
        let guard = ORIGINAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return ApiResult::failure(
                    "No original data snapshot. Please load a file first.",
                )
            }
            Some(df) => df.clone(),
        }
    };

    *GLOBAL_DF.lock().unwrap() = Some(original.clone());
    CLEAN_HISTORY.lock().unwrap().clear();
    sync_active_dataset();
    match df_to_payload(&original, Some(PREVIEW_LIMIT)) {
        Ok(p) => ApiResult::success(p),
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────

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
    type_cols: &[String],
    type_target: &str,
) -> Result<DataFrame> {
    // 1. Column removal
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

    // 2. Row condition filter
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

    // 3. Fill nulls
    if !fillna_col.is_empty() {
        let fill_expr = lit(fillna_val.to_string());
        let will_type_cast_fill_col =
            !type_target.is_empty() && type_cols.iter().any(|c| c == fillna_col);
        let fill_col_expr = if will_type_cast_fill_col {
            col(fillna_col)
                .cast(DataType::String)
                .fill_null(fill_expr)
                .alias(fillna_col)
        } else {
            col(fillna_col).fill_null(fill_expr).alias(fillna_col)
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

    let mut df2 = lf.collect().map_err(|e| anyhow!("{e}"))?;

    // 5. Trim whitespace
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

                df2.replace(c, replaced).map_err(|e| anyhow!("{e}"))?;
            }
        }
    }

    // 7. Type-cast
    if !type_cols.is_empty() {
        for type_col in type_cols {
            let src_dtype = df2
                .column(type_col)
                .map_err(|e| anyhow!("{e}"))?
                .dtype()
                .clone();
            let is_string_src = matches!(src_dtype, DataType::String);

            df2 = match type_target {
                "datetime" if is_string_src => df2
                    .lazy()
                    .with_column(
                        col(type_col.as_str())
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
                            .alias(type_col.as_str()),
                    )
                    .collect()
                    .map_err(|e| anyhow!("datetime parse failed: {e}"))?,
                "date" if is_string_src => df2
                    .lazy()
                    .with_column(
                        col(type_col.as_str())
                            .str()
                            .to_date(StrptimeOptions {
                                format: None,
                                strict: false,
                                exact: true,
                                cache: true,
                            })
                            .alias(type_col.as_str()),
                    )
                    .collect()
                    .map_err(|e| anyhow!("date parse failed: {e}"))?,
                _ => {
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
                    df2.replace(type_col, series.with_name(type_col.clone().into()))
                        .map_err(|e| anyhow!("{e}"))?;
                    df2
                }
            };
        }
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
                "not_contains" => {
                    any_value_to_string(&av).is_some_and(|s| !s.contains(row_filter_val))
                }
                "starts_with" => {
                    any_value_to_string(&av).is_some_and(|s| s.starts_with(row_filter_val))
                }
                "ends_with" => {
                    any_value_to_string(&av).is_some_and(|s| s.ends_with(row_filter_val))
                }
                other => {
                    let _ = other;
                    false
                }
            }
        })
        .collect();

    if !matches!(
        row_filter_op,
        "eq" | "ne"
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
