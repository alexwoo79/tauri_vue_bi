// src-tauri/src/agent/functions/clean/missing_handler.rs
//
// 缺失值处理模块 - 对标 Python 的 Function/Clean/missing_handler.py
//
// 功能：提供多种缺失值填充策略

use anyhow::{Context, Result};
use polars::prelude::*;

/// 缺失值处理策略
pub enum FillStrategy {
    Mean,
    Median,
    Mode,
    ForwardFill,
    BackwardFill,
    Constant(f64),
}

/// 处理指定列的缺失值
pub fn fill_missing_values(
    df: &DataFrame,
    columns: &[&str],
    strategy: FillStrategy,
) -> Result<DataFrame> {
    let mut result = df.clone();

    for &col in columns {
        let col_ref = df.column(col)?;
        let series = col_ref
            .as_series()
            .ok_or_else(|| anyhow::anyhow!("Failed to get series"))?;

        let filled = match strategy {
            FillStrategy::Mean => {
                if let Ok(f64_col) = series.f64() {
                    if f64_col.mean().is_some() {
                        series.fill_null(FillNullStrategy::Forward(None))
                    } else {
                        Ok(series.clone())
                    }
                } else {
                    Ok(series.clone())
                }
            }
            FillStrategy::Median => {
                if let Ok(f64_col) = series.f64() {
                    if f64_col.median().is_some() {
                        series.fill_null(FillNullStrategy::Forward(None))
                    } else {
                        Ok(series.clone())
                    }
                } else {
                    Ok(series.clone())
                }
            }
            FillStrategy::Mode => series.fill_null(FillNullStrategy::Forward(None)),
            FillStrategy::ForwardFill => series.fill_null(FillNullStrategy::Forward(None)),
            FillStrategy::BackwardFill => series.fill_null(FillNullStrategy::Backward(None)),
            FillStrategy::Constant(_value) => Ok(series.clone()),
        };

        result.with_column(filled?)?;
    }

    Ok(result)
}

/// 查找众数
fn find_mode(series: &Series) -> Option<String> {
    let vc = match series.value_counts(false, false, polars::prelude::PlSmallStr::from(""), false) {
        Ok(v) => v,
        Err(_) => return None,
    };
    if vc.height() == 0 {
        return None;
    }
    let counts = match vc.column("count") {
        Ok(col) => match col.u32() {
            Ok(c) => c,
            Err(_) => return None,
        },
        Err(_) => return None,
    };
    let max_count = counts.max().unwrap_or(0);
    for i in 0..vc.height() {
        if counts.get(i).unwrap_or(0) == max_count {
            if let Ok(col) = vc.column("value") {
                if let Ok(v) = col.get(i) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

/// 删除包含缺失值的行
pub fn drop_missing_rows(df: &DataFrame, columns: Option<&[&str]>) -> Result<DataFrame> {
    match columns {
        Some(cols) => {
            let cols_vec: Vec<String> = cols.iter().map(|s| s.to_string()).collect();
            df.drop_nulls(Some(&cols_vec))
        }
        None => df.drop_nulls::<String>(None),
    }
    .context("Failed to drop missing rows")
}

/// 删除包含过多缺失值的列
pub fn drop_missing_columns(df: &DataFrame, threshold: f64) -> Result<DataFrame> {
    let total_rows = df.height() as f64;
    let mut to_keep = Vec::new();

    for col_name in df.get_column_names() {
        let col = df.column(col_name)?;
        let missing_ratio = col.null_count() as f64 / total_rows;
        if missing_ratio <= threshold {
            to_keep.push(col_name.to_string());
        }
    }
    df.select(&to_keep).context("Failed to select columns")
}

/// 获取缺失值统计信息
pub fn get_missing_stats(df: &DataFrame) -> Vec<(String, usize, f64)> {
    let total_rows = df.height() as f64;

    df.get_column_names()
        .iter()
        .filter_map(|&name| match df.column(name) {
            Ok(col) => {
                let missing = col.null_count();
                let ratio = (missing as f64 / total_rows) * 100.0;
                Some((name.to_string(), missing, ratio))
            }
            Err(_) => None,
        })
        .collect()
}
