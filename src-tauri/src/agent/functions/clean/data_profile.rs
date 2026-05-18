// src-tauri/src/agent/functions/clean/data_profile.rs
//
// 数据概况分析模块 - 对标 Python 的 Function/Clean/data_profile.py
//
// 功能：生成数据概况报告，包括列类型、缺失值统计、基本统计量

use anyhow::{Context, Result};
use polars::prelude::*;
use serde_json::json;
use std::collections::HashMap;

/// 列统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ColumnStats {
    pub name: String,
    pub dtype: String,
    pub count: usize,
    pub missing_count: usize,
    pub missing_pct: f64,
    pub unique_count: Option<usize>,
    pub unique_pct: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub std: Option<f64>,
    pub median: Option<f64>,
    pub mode: Option<String>,
    pub sample_values: Vec<String>,
}

/// 数据概况报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct DataProfile {
    pub n_rows: usize,
    pub n_cols: usize,
    pub n_cells: usize,
    pub missing_cells: usize,
    pub missing_pct: f64,
    pub columns: Vec<ColumnStats>,
    pub summary: String,
}

/// 生成数据概况报告
pub fn generate_data_profile(df: &DataFrame) -> Result<DataProfile> {
    let n_rows = df.height();
    let n_cols = df.width();
    let n_cells = n_rows * n_cols;
    
    let mut missing_cells = 0;
    let mut columns: Vec<ColumnStats> = Vec::new();
    
    for col in df.get_columns() {
        let stats = analyze_column(df, col.name(), n_rows);
        missing_cells += stats.missing_count;
        columns.push(stats);
    }
    
    let missing_pct = (missing_cells as f64 / n_cells as f64) * 100.0;
    
    let summary = generate_summary(n_rows, n_cols, missing_cells, &columns);
    
    Ok(DataProfile {
        n_rows,
        n_cols,
        n_cells,
        missing_cells,
        missing_pct,
        columns,
        summary,
    })
}

/// 分析单列统计信息
fn analyze_column(df: &DataFrame, col_name: &str, total_rows: usize) -> ColumnStats {
    let col = df.column(col_name).unwrap();
    let name = col_name.to_string();
    let dtype = col.dtype().to_string();
    let count = col.len();
    let missing_count = col.null_count();
    let missing_pct = (missing_count as f64 / total_rows as f64) * 100.0;
    
    let dt = col.dtype();
    let is_num = dt.is_integer() || dt.is_float();
    let is_str = dt.is_string();
    
    // 获取唯一值统计
    let (unique_count, unique_pct) = if is_num || is_str {
        let unique = col.n_unique().unwrap_or(0);
        (Some(unique), Some((unique as f64 / total_rows as f64) * 100.0))
    } else {
        (None, None)
    };
    
    // ✅ 修复：新版空数组创建
    let (min, max, mean, std, median) = if is_num {
        match col.f64() {
            Ok(f64_col) => (
                f64_col.min(),
                f64_col.max(),
                f64_col.mean(),
                f64_col.std(QuantileMethod::Nearest),
                f64_col.median(),
            ),
            Err(_) => (None, None, None, None, None),
        }
    } else {
        (None, None, None, None, None)
    };
    
    // 计算众数
    let mode = if is_str || is_num {
        match col.as_series() {
            Some(series) => match series.value_counts(false, false, None) {
                Ok(vc) => {
                    if vc.height() > 0 {
                        match vc.column("value") {
                            Ok(val_col) => match val_col.get(0) {
                                Ok(Some(val)) => Some(format!("{}", val)),
                                _ => None,
                            },
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            },
            None => None,
        }
    } else {
        None
    };
    
    // 获取样本值
    let sample_values: Vec<String> = (0..col.len().min(5))
        .filter_map(|i| {
            match col.get(i) {
                Ok(v) => Some(format!("{:?}", v)),
                Err(_) => None,
            }
        })
        .collect();
    
    ColumnStats {
        name,
        dtype,
        count,
        missing_count,
        missing_pct,
        unique_count,
        unique_pct,
        min,
        max,
        mean,
        std,
        median,
        mode,
        sample_values,
    }
}

/// 生成数据概况摘要
fn generate_summary(n_rows: usize, n_cols: usize, missing_cells: usize, columns: &[ColumnStats]) -> String {
    let mut summary = String::new();
    
    summary.push_str(&format!("数据集包含 {} 行、{} 列，共 {} 个单元格。\n", n_rows, n_cols, n_rows * n_cols));
    
    let missing_pct = (missing_cells as f64 / (n_rows * n_cols) as f64) * 100.0;
    summary.push_str(&format!("缺失值占比: {:.2}% ({} 个单元格)\n", missing_pct, missing_cells));
    
    let num_cols = columns.iter().filter(|c| c.dtype.contains("Int") || c.dtype.contains("Float")).count();
    let str_cols = columns.iter().filter(|c| c.dtype.contains("String")).count();
    let date_cols = columns.iter().filter(|c| c.dtype.contains("Date") || c.dtype.contains("Time")).count();
    
    summary.push_str(&format!("列类型分布: 数值型 {} 列, 字符串 {} 列, 日期时间 {} 列\n", num_cols, str_cols, date_cols));
    
    let high_missing_cols = columns.iter().filter(|c| c.missing_pct > 50.0).count();
    if high_missing_cols > 0 {
        summary.push_str(&format!("警告: {} 列缺失值超过 50%\n", high_missing_cols));
    }
    
    summary
}

/// 获取数值列列表
pub fn get_numeric_columns(df: &DataFrame) -> Vec<String> {
    let mut result = Vec::new();
    for col in df.get_columns() {
        let dt = col.dtype();
        if dt.is_integer() || dt.is_float() {
            result.push(col.name().to_string());
        }
    }
    result
}

/// 获取字符串列列表
pub fn get_string_columns(df: &DataFrame) -> Vec<String> {
    let mut result = Vec::new();
    for col in df.get_columns() {
        if col.dtype().is_string() {
            result.push(col.name().to_string());
        }
    }
    result
}

/// 获取日期列列表
pub fn get_date_columns(df: &DataFrame) -> Vec<String> {
    let mut result = Vec::new();
    for col in df.get_columns() {
        if col.dtype().is_temporal() {
            result.push(col.name().to_string());
        }
    }
    result
}
