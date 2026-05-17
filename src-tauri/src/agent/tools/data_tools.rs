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

/// 获取数据源结构（参考 Python 版本格式，包含表名）
pub fn tool_get_schema() -> Result<String> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;
    
    let mut schema_parts = Vec::new();
    
    // 表名固定为 "data"（与 SQLContext 注册名一致）
    schema_parts.push(format!("Table: data  ({} rows)", df.height()));
    
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
    
    Ok(schema_parts.join("\n"))
}

const MAX_DISPLAY_ROWS: usize = 200;  // 与 Python 版本保持一致

/// 执行 SQL 查询（使用 Polars SQL 上下文，参考 Python format_result 格式）
pub fn tool_query_data(sql: &str) -> Result<String> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;
    
    // 创建 SQL 上下文并注册 DataFrame
    let mut ctx = SQLContext::new();
    ctx.register("data", df.clone().lazy());
    
    // 执行 SQL 查询
    let result = ctx
        .execute(sql)
        .map_err(|e| anyhow::anyhow!("SQL Error: {}", e))?
        .collect()
        .context("SQL 查询结果收集失败")?;
    
    // 处理空结果
    if result.height() == 0 {
        return Ok("Query returned no results.".to_string());
    }
    
    // 获取预览数据（最多 200 行）
    let total_rows = result.height();
    let preview = if total_rows > MAX_DISPLAY_ROWS {
        result.head(Some(MAX_DISPLAY_ROWS))
    } else {
        result.clone()
    };
    
    // 格式化为表格字符串（参考 Python 的 to_string(index=False, max_cols=30)）
    let table_str = format_dataframe(&preview);
    
    // 添加行数提示
    let mut output = table_str;
    if total_rows > MAX_DISPLAY_ROWS {
        output.push_str(&format!(
            "\n\n... showing {} of {} rows",
            MAX_DISPLAY_ROWS,
            total_rows
        ));
    }
    
    Ok(output)
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

/// 格式化 DataFrame 为文本（参考 Python pandas to_string(index=False, max_cols=30)）
fn format_dataframe(df: &DataFrame) -> String {
    let max_cols = 30;
    let cols_to_display = std::cmp::min(df.width(), max_cols);
    
    // 获取列名
    let col_names = df.get_column_names();
    let headers: Vec<&str> = col_names
        .iter()
        .take(cols_to_display)
        .map(|s| s.as_str())
        .collect();
    
    let header_line = headers.join("  ");
    let mut lines = vec![header_line];
    
    // 显示所有行（已由调用者处理行数限制）
    let columns = df.get_columns();
    for row_idx in 0..df.height() {
        let mut row_values = Vec::new();
        for col_idx in 0..cols_to_display {
            let col = &columns[col_idx];
            let value = col.get(row_idx)
                .map(|av| match av {
                    AnyValue::Null => "null".to_string(),
                    AnyValue::String(s) => s.to_string(),
                    AnyValue::Int32(v) => v.to_string(),
                    AnyValue::Int64(v) => v.to_string(),
                    AnyValue::Float32(v) => format!("{:.2}", v),
                    AnyValue::Float64(v) => format!("{:.2}", v),
                    AnyValue::Boolean(v) => v.to_string(),
                    _ => format!("{:?}", av),
                })
                .unwrap_or_else(|_| "null".to_string());
            row_values.push(value);
        }
        lines.push(row_values.join("  "));
    }
    
    lines.join("\n")
}
