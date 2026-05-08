// src-tauri/src/commands/groupby.rs
//
// 分组聚合命令（GroupBy Aggregation Command）

use anyhow::{anyhow, bail, Result};
use polars::prelude::*;

use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::GLOBAL_DF;
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn groupby_agg(
    group_cols: Vec<String>,
    agg_col: String,
    agg_func: String,
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
    match groupby_agg_impl(&df, &group_cols, &agg_col, &agg_func) {
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
        .group_by(
            group_cols
                .iter()
                .map(|c| col(c.as_str()))
                .collect::<Vec<_>>(),
        )
        .agg([agg_expr])
        .sort(
            group_cols.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            SortMultipleOptions::default(),
        )
        .collect()
        .map_err(|e| anyhow!("{e}"))?;

    Ok(result)
}
