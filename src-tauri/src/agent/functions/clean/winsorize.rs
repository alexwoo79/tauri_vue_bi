// src-tauri/src/agent/functions/clean/winsorize.rs
//
// Winsorize 处理模块 - 对标 Python 的 Function/Clean/winsorize.py
//
// 功能：对极端值进行 Winsorize 处理（将极端值替换为分位数边界值）

use anyhow::{Context, Result};
use polars::prelude::*;

/// 对指定列进行 Winsorize 处理
pub fn winsorize(
    df: &DataFrame,
    columns: &[&str],
    lower_pct: f64,
    upper_pct: f64,
) -> Result<DataFrame> {
    let mut result = df.clone();
    
    for &col in columns {
        let col_ref = df.column(col)?;
        let dt = col_ref.dtype();
        
        if !dt.is_integer() && !dt.is_float() {
            continue;
        }
        
        // Polars 0.46 兼容：转换为 Series 后处理
        let series = col_ref.as_series();
        let f64_series = series.f64()?;
        
        let lower_bound = f64_series.quantile(lower_pct / 100.0, QuantileInterpolOptions::default())?;
        let upper_bound = f64_series.quantile(upper_pct / 100.0, QuantileInterpolOptions::default())?;
        
        let lower_val = lower_bound.unwrap_or(f64::NEG_INFINITY);
        let upper_val = upper_bound.unwrap_or(f64::INFINITY);
        
        // 手动实现 clamp 功能
        let winsorized = f64_series.apply(|v: f64| v.max(lower_val).min(upper_val));
        let winsorized_series = winsorized.into_series();
        
        result.with_column(winsorized_series)?;
    }
    
    Ok(result)
}

/// 对指定列进行单侧 Winsorize 处理
pub fn winsorize_one_sided(
    df: &DataFrame,
    columns: &[&str],
    pct: f64,
    side: &str,
) -> Result<DataFrame> {
    match side {
        "lower" => winsorize(df, columns, pct, 100.0),
        "upper" => winsorize(df, columns, 0.0, pct),
        _ => Err(anyhow::anyhow!("Invalid side: {}", side)),
    }
}
