// src-tauri/src/agent/functions/clean/trimming.rs
//
// Trimming 处理模块 - 对标 Python 的 Function/Clean/trimming.py
//
// 功能：对极端值进行 Trimming 处理（删除极端值所在的行）

use anyhow::{Result};
use polars::prelude::*;
use polars::prelude::QuantileMethod;
/// 对指定列进行 Trimming 处理
pub fn trim(
    df: &DataFrame,
    columns: &[&str],
    lower_pct: f64,
    upper_pct: f64,
) -> Result<DataFrame> {
    let mut df = df.clone();
    
    for &col in columns {
        if let Ok(col_ref) = df.column(col) {
            let dt = col_ref.dtype();
            if !dt.is_integer() && !dt.is_float() {
                continue;
            }
            
            // Polars 0.46 兼容：使用 f64 系列的 quantile 方法
            let f64_col = col_ref.f64()?;
            let lower_bound = f64_col.quantile(lower_pct / 100.0, QuantileMethod::default())?;
            let upper_bound = f64_col.quantile(upper_pct / 100.0, QuantileMethod::default())?;
            
            let lower_val = lower_bound.unwrap_or(f64::NEG_INFINITY);
            let upper_val = upper_bound.unwrap_or(f64::INFINITY);
            
            // 创建掩码
            let mask_lower = f64_col.gt_eq(lower_val);
            let mask_upper = f64_col.lt_eq(upper_val);
            let mask = mask_lower & mask_upper;
            
            df = df.filter(&mask)?;
        }
    }
    
    Ok(df)
}

/// 对指定列进行单侧 Trimming 处理
pub fn trim_one_sided(
    df: &DataFrame,
    columns: &[&str],
    pct: f64,
    side: &str,
) -> Result<DataFrame> {
    match side {
        "lower" => trim(df, columns, pct, 100.0),
        "upper" => trim(df, columns, 0.0, pct),
        _ => Err(anyhow::anyhow!("Invalid side: {}", side)),
    }
}
