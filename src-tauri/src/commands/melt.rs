// src-tauri/src/commands/melt.rs
//
// 逆透视（Melt / Unpivot）命令

use anyhow::{anyhow, bail, Result};
use polars::prelude::*;

use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::{register_dataset, GLOBAL_DF};
use crate::types::{ApiResult, ChartPayload};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn melt_data(
    id_vars: Vec<String>,
    value_vars: Vec<String>,
    var_name: String,
    value_name: String,
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
    match melt_data_impl(&df, &id_vars, &value_vars, &var_name, &value_name) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let default_name = format!("逆透视结果_{}", value_name);
                register_dataset(
                    &result_df,
                    dataset_name.unwrap_or(default_name),
                    "melt_data".to_string(),
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

pub fn melt_data_impl(
    df: &DataFrame,
    id_vars: &[String],
    value_vars: &[String],
    var_name: &str,
    value_name: &str,
) -> Result<DataFrame> {
    let all_cols: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|n| n.to_string())
        .collect();

    for c in id_vars {
        if !all_cols.iter().any(|n| n == c) {
            bail!("id column not found: {c}");
        }
    }

    let selected_value_vars: Vec<String> = if value_vars.is_empty() {
        all_cols
            .iter()
            .filter(|n| !id_vars.iter().any(|id| id == *n))
            .cloned()
            .collect()
    } else {
        for c in value_vars {
            if !all_cols.iter().any(|n| n == c) {
                bail!("value column not found: {c}");
            }
        }
        value_vars.to_vec()
    };

    if selected_value_vars.is_empty() {
        bail!("No columns available to melt. Please choose at least one value column.");
    }

    if selected_value_vars
        .iter()
        .any(|v| id_vars.iter().any(|id| id == v))
    {
        bail!("id_vars and value_vars must not overlap");
    }

    let mut out: Option<DataFrame> = None;
    for v in &selected_value_vars {
        let mut select_cols: Vec<&str> = id_vars.iter().map(|c| c.as_str()).collect();
        select_cols.push(v.as_str());

        let mut part = df.select(select_cols).map_err(|e| anyhow!("{e}"))?;
        part.rename(v, value_name.into())
            .map_err(|e| anyhow!("{e}"))?;

        let var_series = Series::new(var_name.into(), vec![v.as_str(); part.height()]);
        part.with_column(var_series).map_err(|e| anyhow!("{e}"))?;

        let mut final_cols: Vec<&str> = id_vars.iter().map(|c| c.as_str()).collect();
        final_cols.push(var_name);
        final_cols.push(value_name);
        let part = part.select(final_cols).map_err(|e| anyhow!("{e}"))?;

        match out.as_mut() {
            Some(acc) => {
                acc.vstack_mut(&part).map_err(|e| anyhow!("{e}"))?;
            }
            None => out = Some(part),
        }
    }

    match out {
        Some(df) => Ok(df),
        None => bail!("melt failed: empty result"),
    }
}
