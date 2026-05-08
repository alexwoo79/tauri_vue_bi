// src-tauri/src/commands/chart.rs
//
// 图表数据命令（Chart Data Command）

use anyhow::{anyhow, bail, Result};
use polars::prelude::*;

use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::GLOBAL_DF;
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_chart_data(
    x_col: String,
    y_cols: Vec<String>,
    color_col: Option<String>,
    sort_by: String,
    sort_asc: bool,
    top_n: i64,
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
    match fetch_chart_data_impl(&df, &x_col, &y_cols, color_col.as_deref(), &sort_by, sort_asc, top_n) {
        Ok(result_df) => match df_to_payload(&result_df, Some(CHART_LIMIT)) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn fetch_chart_data_impl(
    df: &DataFrame,
    x_col: &str,
    y_cols: &[String],
    color_col: Option<&str>,
    sort_by: &str,
    sort_asc: bool,
    top_n: i64,
) -> Result<DataFrame> {
    if y_cols.is_empty() {
        bail!("At least one y column is required");
    }

    let mut keep: Vec<&str> = vec![x_col];
    for y in y_cols {
        if !keep.contains(&y.as_str()) {
            keep.push(y.as_str());
        }
    }
    if let Some(c) = color_col {
        if !keep.contains(&c) {
            keep.push(c);
        }
    }

    let mut result = df.select(keep).map_err(|e| anyhow!("{e}"))?;

    let primary_y = y_cols[0].as_str();
    let sort_col = match sort_by {
        "x" => Some(x_col),
        "y" => Some(primary_y),
        _ => None,
    };
    if let Some(col_name) = sort_col {
        result = result
            .sort(
                [col_name],
                SortMultipleOptions::default().with_order_descending(!sort_asc),
            )
            .map_err(|e| anyhow!("{e}"))?;
    }

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
