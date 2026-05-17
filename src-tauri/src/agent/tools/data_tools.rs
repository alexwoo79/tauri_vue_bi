// src-tauri/src/agent/tools/data_tools.rs
//
// 数据操作工具 - Data Query and Analysis Tools
//
// 提供以下功能：
// - get_schema: 获取数据源结构
// - query_data: 执行 SQL 查询
// - run_analysis: 运行统计分析
// - generate_chart: 生成图表
// - profile_data: 数据概况分析
// - clean_data: 数据清洗

use anyhow::{Context, Result};
use polars::prelude::*;
use polars::sql::SQLContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::GLOBAL_DF;

/// 字段映射配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub x: Option<String>,
    pub y: Option<String>,
    pub series: Option<String>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub size: Option<String>,
    pub color: Option<String>,
    pub group: Option<String>,
    pub order: Option<String>,
    pub dimensions: Option<Vec<String>>,
}

/// 图表生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResult {
    pub html: String,
    pub chart_type: String,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}

/// 获取数据源结构
pub fn tool_get_schema() -> Result<String> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;
    
    let mut schema_parts = Vec::new();
    
    for field in df.schema().iter_fields() {
        let dtype_str = match field.dtype() {
            DataType::Int32 | DataType::Int64 => "INTEGER",
            DataType::Float32 | DataType::Float64 => "FLOAT",
            DataType::String => "STRING",
            DataType::Boolean => "BOOLEAN",
            DataType::Date => "DATE",
            DataType::Datetime(_, _) => "DATETIME",
            _ => "OTHER",
        };
        schema_parts.push(format!("  {}  {}", field.name(), dtype_str));
    }
    
    Ok(format!(
        "DataFrame Schema ({} rows, {} columns):\n{}",
        df.height(),
        df.width(),
        schema_parts.join("\n")
    ))
}

/// 执行 SQL 查询（使用 Polars SQL 上下文）
pub fn tool_query_data(sql: &str) -> Result<String> {
    Err(anyhow::anyhow!("Polars SQL support not available in this build (tool_query_data not implemented)."))
}

/// 运行分析（分位数分析、决策树等）
pub fn tool_run_analysis(
    analysis_name: &str,
    _sql: &str,
    target_column: &str,
    _groupby_column: Option<&str>,
    n_deciles: usize,
) -> Result<String> {
    // TODO: 实现具体的分析逻辑
    // 这里需要集成 Function/Analyze 目录中的 Python 算法
    
    match analysis_name {
        "decile" => Ok(format!("Decile analysis on '{}' with {} deciles", target_column, n_deciles)),
        "decision_tree" => Ok("Decision tree analysis (TODO: implement)".to_string()),
        "kmeans" => Ok("K-Means clustering (TODO: implement)".to_string()),
        _ => Err(anyhow::anyhow!("Unknown analysis: {}", analysis_name)),
    }
}

/// 生成图表
pub fn tool_generate_chart(
    chart_type: &str,
    _sql: &str,
    _field_mapping: FieldMapping,
    title: &str,
    color_scheme: &str,
) -> Result<ChartResult> {
    // TODO: 集成图表生成模块
    // 需要迁移 Function/Charts_generation 中的 Rust 实现
    
    Ok(ChartResult {
        html: format!("<div>Chart placeholder for {}</div>", chart_type),
        chart_type: chart_type.to_string(),
        warnings: vec!["Chart generation not yet implemented in Rust".to_string()],
        meta: serde_json::json!({
            "title": title,
            "color_scheme": color_scheme,
        }),
    })
}

/// 数据概况分析
pub fn tool_profile_data(table_name: Option<&str>, columns: Option<Vec<String>>) -> Result<serde_json::Value> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;
    
    let mut profile = serde_json::Map::new();
    
    // 基本统计
    profile.insert("rows".to_string(), serde_json::json!(df.height()));
    profile.insert("columns".to_string(), serde_json::json!(df.width()));
    
    // 列信息
    let mut col_info = Vec::new();
    for field in df.schema().iter_fields() {
        let null_count = df.column(field.name())
            .map(|col| col.null_count())
            .unwrap_or(0);
        
        col_info.push(serde_json::json!({
            "name": field.name(),
            "dtype": format!("{:?}", field.dtype()),
            "null_count": null_count,
        }));
    }
    profile.insert("columns_detail".to_string(), serde_json::json!(col_info));
    
    Ok(serde_json::Value::Object(profile))
}

/// 数据清洗
pub fn tool_clean_data(
    operation: &str,
    table_name: Option<&str>,
    columns: Option<Vec<String>>,
    fill_method: &str,
    lower_pct: f64,
    upper_pct: f64,
) -> Result<String> {
    let mut df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_mut().context("没有加载的数据集")?;
    
    match operation {
        "remove_duplicates" => {
            *df = df.unique_stable(None, UniqueKeepStrategy::First, None)?;
            Ok("已删除重复行".to_string())
        }
        "fill_missing" => {
            // TODO: 实现缺失值填充
            Ok(format!("Fill missing values using method: {}", fill_method))
        }
        "winsorize" => {
            // TODO: 实现 Winsorize（截尾处理）
            Ok(format!("Winsorize at {}% and {}%", lower_pct, upper_pct))
        }
        "trim" => {
            // TODO: 实现异常值修剪
            Ok("Trim outliers".to_string())
        }
        _ => Err(anyhow::anyhow!("Unknown clean operation: {}", operation)),
    }
}

/// 格式化 DataFrame 为文本
fn format_dataframe(df: &DataFrame) -> String {
    let headers: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
    let header_line = headers.join("\t");
    
    let mut lines = vec![header_line];
    
    for row_idx in 0..std::cmp::min(df.height(), 50) {
        let mut row_values = Vec::new();
        for col in df.get_columns() {
            let value = col.get(row_idx).map(|av| format!("{:?}", av)).unwrap_or_else(|_| "null".to_string());
            row_values.push(value);
        }
        lines.push(row_values.join("\t"));
    }
    
    lines.join("\n")
}
