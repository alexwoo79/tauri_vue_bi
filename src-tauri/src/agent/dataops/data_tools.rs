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

use crate::agent::functions::{
    analyze::{run_decile_analysis, run_decision_tree, run_kmeans},
    clean::{fill_missing_values, generate_data_profile, trim, winsorize, FillStrategy},
};
use crate::state::GLOBAL_DF;

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

const MAX_DISPLAY_ROWS: usize = 200; // 与 Python 版本保持一致

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
            MAX_DISPLAY_ROWS, total_rows
        ));
    }

    Ok(output)
}

/// 运行分析（分位数分析、决策树等）
pub fn tool_run_analysis(
    analysis_name: &str,
    _sql: &str,
    target_column: &str,
    groupby_column: Option<&str>,
    n_deciles: usize,
) -> Result<String> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;

    match analysis_name {
        "decile" => {
            let result = run_decile_analysis(df, target_column, groupby_column, n_deciles)?;
            Ok(result.markdown)
        }
        "decision_tree" => {
            let feature_columns: Vec<String> = df
                .get_column_names()
                .iter()
                .filter(|&&c| c != target_column)
                .map(|s| s.to_string())
                .collect();
            let result = run_decision_tree(df, target_column, Some(feature_columns), 5)?;
            Ok(result.markdown)
        }
        "kmeans" => {
            let feature_columns: Vec<String> = df
                .get_column_names()
                .iter()
                .filter(|&&c| c != target_column)
                .map(|s| s.to_string())
                .collect();
            let result = run_kmeans(df, feature_columns, n_deciles, 100)?;
            Ok(result.markdown)
        }
        _ => Err(anyhow::anyhow!("Unknown analysis: {}", analysis_name)),
    }
}

/// 数据概况分析
pub fn tool_profile_data(
    table_name: Option<&str>,
    columns: Option<Vec<String>>,
) -> Result<serde_json::Value> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;

    let result = generate_data_profile(df)?;

    serde_json::to_value(result).map_err(|e| anyhow::anyhow!("Failed to serialize profile: {}", e))
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

    let col_refs: Vec<String> = columns.unwrap_or_default();
    let col_str_refs: Vec<&str> = col_refs.iter().map(|s| s.as_str()).collect();
    let col_slice: &[&str] = col_str_refs.as_slice();

    match operation {
        "remove_duplicates" => {
            *df = df.unique_stable(None, UniqueKeepStrategy::First, None)?;
            Ok("已删除重复行".to_string())
        }
        "fill_missing" => {
            let strategy = match fill_method {
                "mean" => FillStrategy::Mean,
                "median" => FillStrategy::Median,
                "mode" => FillStrategy::Mode,
                "ffill" => FillStrategy::ForwardFill,
                "bfill" => FillStrategy::BackwardFill,
                _ => FillStrategy::Mean,
            };
            *df = fill_missing_values(df, col_slice, strategy)?;
            Ok(format!("使用 {} 方法填充缺失值", fill_method))
        }
        "winsorize" => {
            *df = winsorize(df, col_slice, lower_pct, upper_pct)?;
            Ok(format!(
                "已在 {}% 和 {}% 处进行缩尾处理",
                lower_pct, upper_pct
            ))
        }
        "trim" => {
            *df = trim(df, col_slice, lower_pct, upper_pct)?;
            Ok(format!(
                "已在 {}% 和 {}% 处截断异常值",
                lower_pct, upper_pct
            ))
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
            let value = col
                .get(row_idx)
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
