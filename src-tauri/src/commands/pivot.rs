// src-tauri/src/commands/pivot.rs
//
// 透视表命令（Pivot Table Command）

use anyhow::{anyhow, bail, Result};
use polars::prelude::*;
use polars_ops::frame::pivot::{pivot, PivotAgg};

use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::{register_dataset, GLOBAL_DF};
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn pivot_data(
    rows: Vec<String>,
    columns: Vec<String>,
    values: Vec<String>,
    agg: String,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
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
    match pivot_data_impl(&df, &rows, &columns, &values, &agg) {
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

// ─────────────────────────────────────────────────────────────────────────────
// Implementation
// ─────────────────────────────────────────────────────────────────────────────

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
        let agg_exprs: Vec<Expr> = values
            .iter()
            .map(|v| {
                match agg {
                    "sum" => col(v.as_str()).sum(),
                    "mean" => col(v.as_str()).mean(),
                    "count" => col(v.as_str()).count(),
                    "min" => col(v.as_str()).min(),
                    "max" => col(v.as_str()).max(),
                    _ => unreachable!(),
                }
                .alias(v.as_str())
            })
            .collect();

        let result = df
            .clone()
            .lazy()
            .group_by(rows.iter().map(|c| col(c.as_str())).collect::<Vec<_>>())
            .agg(agg_exprs)
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
