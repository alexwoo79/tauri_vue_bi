// src-tauri/src/agent/functions/chart_generate.rs
//
// 图表生成入口 - 统一调度所有图表类型
// 严格对标 Python Data-Analysis-Agent 的 chart_generate.py

use anyhow::{Result};
use polars::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use super::base::*;
use super::bar_chart::*;
use super::grouped_bar_chart::*;
use super::stacked_bar_chart::*;
use super::line_chart::*;
use super::area_chart::*;
use super::stacked_area_chart::*;
use super::pie_chart::*;
use super::histogram::*;
use super::box_plot::*;
use super::violin_chart::*;
use super::scatter_plot::*;
use super::bubble_plot::*;
use super::heatmap::*;
use super::radar_chart::*;
use super::sankey_chart::*;
use super::waterfall::*;
use super::nightingale_chart::*;

/// 生成图表的主要入口函数
/// 对标 Python 的 generate_chart()
pub fn generate_chart(
    df: Option<&DataFrame>,
    chart_type: &str,
    mapping: Option<FieldMapping>,
    options: Option<ChartOptions>,
    color_scheme: Option<&str>,
) -> Result<ChartResult> {
    // 检查数据
    let df = df.ok_or_else(|| anyhow::anyhow!("No data"))?;
    
    if df.is_empty() {
        return Ok(ChartResult::error(vec!["No data".to_string()]));
    }

    // 统一列名类型为字符串（Polars 0.46 兼容）
    let df = df.clone();

    // 自动检测字段映射（如果未提供）
    let mapping = if let Some(m) = mapping {
        m
    } else {
        auto_detect_mapping(&df, chart_type)
    };

    // 合并 options，添加 color_scheme
    let mut merged_options = options.unwrap_or_default();
    merged_options.color_scheme = color_scheme.map(|s| s.to_string()).or(merged_options.color_scheme);

    // 将 DataFrame 转换为 Vec<HashMap>
    let data = dataframe_to_hashmaps(&df)?;

    // 调用具体的图表生成函数
    let result = match chart_type {
        "bar_chart" => generate_bar_chart(data, mapping, merged_options),
        "grouped_bar" => generate_grouped_bar_chart(data, mapping, merged_options),
        "stacked_bar" => generate_stacked_bar_chart(data, mapping, merged_options),
        "line_chart" => generate_line_chart(data, mapping, merged_options),
        "area_chart" => generate_area_chart(data, mapping, merged_options),
        "stacked_area" => generate_stacked_area_chart(data, mapping, merged_options),
        "heatmap" => generate_heatmap(data, mapping, merged_options),
        "violin_chart" => generate_violin_chart(data, mapping, merged_options),
        "box_plot" => generate_box_plot(data, mapping, merged_options),
        "histogram" => generate_histogram(data, mapping, merged_options),
        "waterfall" => generate_waterfall(data, mapping, merged_options),
        "sankey" => generate_sankey_chart(data, mapping, merged_options),
        "scatter_plot" => generate_scatter_plot(data, mapping, merged_options),
        "bubble_plot" => generate_bubble_plot(data, mapping, merged_options),
        "pie_chart" => generate_pie_chart(data, mapping, merged_options),
        "radar_chart" => generate_radar_chart(data, mapping, merged_options),
        "nightingale" => generate_nightingale_chart(data, mapping, merged_options),
        _ => {
            return Ok(ChartResult::error(vec![format!("Unknown chart type: {}", chart_type)]));
        }
    };

    // 验证结果
    match result {
        Ok(chart_result) => {
            if chart_result.is_valid() {
                Ok(chart_result)
            } else {
                let html_len = chart_result.html.len();
                let msg = if !chart_result.warnings.is_empty() {
                    chart_result.warnings[0].clone()
                } else {
                    format!("Generated chart is invalid (html={} chars)", html_len)
                };
                Ok(ChartResult::error(vec![msg]))
            }
        }
        Err(e) => Ok(ChartResult::error(vec![format!("Chart generation error: {}", e)])),
    }
}

/// 自动检测字段映射
/// 对标 Python 的 _auto_detect_mapping()
pub fn auto_detect_mapping(df: &DataFrame, chart_type: &str) -> FieldMapping {
    let numeric_cols: Vec<String> = df.get_column_names()
        .iter()
        .filter(|&&name| {
            if let Ok(series) = df.column(name) {
                let dt = series.dtype();
                dt.is_integer() || dt.is_float()
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();
    
    let string_cols: Vec<String> = df.get_column_names()
        .iter()
        .filter(|&&name| {
            if let Ok(series) = df.column(name) {
                series.dtype().is_string()
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();

    let mut mapping = FieldMapping::default();

    match chart_type {
        "bar_chart" | "grouped_bar" | "stacked_bar" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
                if string_cols.len() > 1 {
                    mapping.series = Some(string_cols[1].clone());
                } else if numeric_cols.len() > 1 {
                    mapping.series = Some(numeric_cols[1].clone());
                }
            }
        }
        
        "line_chart" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            } else if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
            }
        }
        
        "scatter_plot" => {
            if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
                if numeric_cols.len() >= 3 {
                    mapping.size = Some(numeric_cols[2].clone());
                }
            }
        }
        
        "pie_chart" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.label = Some(string_cols[0].clone());
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        
        "heatmap" => {
            if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
                if numeric_cols.len() >= 3 {
                    mapping.value = Some(numeric_cols[2].clone());
                }
            }
        }
        
        "histogram" => {
            if !numeric_cols.is_empty() {
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        
        "box_plot" | "violin_chart" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            } else if !numeric_cols.is_empty() {
                mapping.y = Some(numeric_cols[0].clone());
            }
        }
        
        "waterfall" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            }
        }
        
        "sankey" => {
            if string_cols.len() >= 2 {
                mapping.source = Some(string_cols[0].clone());
                mapping.target = Some(string_cols[1].clone());
                if !numeric_cols.is_empty() {
                    mapping.value = Some(numeric_cols[0].clone());
                }
            }
        }
        
        "bubble_plot" => {
            if numeric_cols.len() >= 3 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
                mapping.size = Some(numeric_cols[2].clone());
            }
        }
        
        "radar_chart" => {
            if !numeric_cols.is_empty() {
                mapping.dimensions = Some(numeric_cols.clone());
                if !string_cols.is_empty() {
                    mapping.label = Some(string_cols[0].clone());
                }
            }
        }
        
        "nightingale" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.label = Some(string_cols[0].clone());
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        
        _ => {}
    }

    mapping
}

/// 推荐图表类型
/// 对标 Python 的 recommend_charts()
pub fn recommend_charts(df: &DataFrame, limit: usize) -> Vec<ChartRecommendation> {
    let numeric_cols: Vec<String> = df.get_column_names()
        .iter()
        .filter(|&&name| {
            if let Ok(series) = df.column(name) {
                let dt = series.dtype();
                dt.is_integer() || dt.is_float()
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();
    
    let string_cols: Vec<String> = df.get_column_names()
        .iter()
        .filter(|&&name| {
            if let Ok(series) = df.column(name) {
                series.dtype().is_string()
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();

    let mut recommendations = Vec::new();

    // 基于数据特征推荐图表
    if !string_cols.is_empty() && !numeric_cols.is_empty() {
        recommendations.push(ChartRecommendation { chart_id: "bar_chart".to_string(), score: 0.95 });
        recommendations.push(ChartRecommendation { chart_id: "grouped_bar".to_string(), score: 0.90 });
        recommendations.push(ChartRecommendation { chart_id: "line_chart".to_string(), score: 0.85 });
        recommendations.push(ChartRecommendation { chart_id: "pie_chart".to_string(), score: 0.70 });
    }

    if numeric_cols.len() >= 2 {
        recommendations.push(ChartRecommendation { chart_id: "scatter_plot".to_string(), score: 0.80 });
        recommendations.push(ChartRecommendation { chart_id: "heatmap".to_string(), score: 0.75 });
        if numeric_cols.len() >= 3 {
            recommendations.push(ChartRecommendation { chart_id: "bubble_plot".to_string(), score: 0.72 });
        }
    }

    if !numeric_cols.is_empty() {
        recommendations.push(ChartRecommendation { chart_id: "histogram".to_string(), score: 0.78 });
        recommendations.push(ChartRecommendation { chart_id: "box_plot".to_string(), score: 0.73 });
    }

    // 按分数排序并限制数量
    recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    recommendations.truncate(limit);

    recommendations
}

/// 图表推荐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartRecommendation {
    pub chart_id: String,
    pub score: f64,
}

/// 将 DataFrame 转换为 Vec<HashMap<String, serde_json::Value>>
fn dataframe_to_hashmaps(df: &DataFrame) -> Result<Vec<HashMap<String, serde_json::Value>>> {
    let n_rows = df.height();
    let column_names = df.get_column_names();
    
    let mut result = Vec::with_capacity(n_rows);
    
    for row_idx in 0..n_rows {
        let mut row_map = HashMap::new();
        
        for col_name in column_names.iter() {
            let column = df.column(col_name)?;
            let value = match column.dtype() {
                DataType::String => column.str()?.get(row_idx).map(|s| serde_json::Value::String(s.to_string())),
                DataType::Int64 | DataType::Int32 | DataType::Int16 | DataType::Int8 => {
                    column.i64()?.get(row_idx).map(|v| serde_json::Value::Number(serde_json::Number::from(v)))
                }
                DataType::Float64 | DataType::Float32 => {
                    column.f64()?.get(row_idx).and_then(|v| serde_json::Number::from_f64(v).map(serde_json::Value::Number))
                }
                DataType::Boolean => column.bool()?.get(row_idx).map(serde_json::Value::Bool),
                _ => {
                    // 其他类型转换为字符串（Polars 0.46 兼容）
                    column.str()?.get(row_idx).map(|s| serde_json::Value::String(s.to_string()))
                }
            };
            
            if let Some(val) = value {
                row_map.insert(col_name.to_string(), val);
            } else {
                row_map.insert(col_name.to_string(), serde_json::Value::Null);
            }
        }
        
        result.push(row_map);
    }
    
    Ok(result)
}

/// 简化版本的生成函数（用于从 agent 调用）
pub fn generate_chart_simple(
    chart_type: &str,
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    // 直接调用具体图表生成函数
    let result = match chart_type {
        "bar_chart" => generate_bar_chart(data, mapping, options),
        "grouped_bar" => generate_grouped_bar_chart(data, mapping, options),
        "stacked_bar" => generate_stacked_bar_chart(data, mapping, options),
        "line_chart" => generate_line_chart(data, mapping, options),
        "area_chart" => generate_area_chart(data, mapping, options),
        "stacked_area" => generate_stacked_area_chart(data, mapping, options),
        "heatmap" => generate_heatmap(data, mapping, options),
        "violin_chart" => generate_violin_chart(data, mapping, options),
        "box_plot" => generate_box_plot(data, mapping, options),
        "histogram" => generate_histogram(data, mapping, options),
        "waterfall" => generate_waterfall(data, mapping, options),
        "sankey" => generate_sankey_chart(data, mapping, options),
        "scatter_plot" => generate_scatter_plot(data, mapping, options),
        "bubble_plot" => generate_bubble_plot(data, mapping, options),
        "pie_chart" => generate_pie_chart(data, mapping, options),
        "radar_chart" => generate_radar_chart(data, mapping, options),
        "nightingale" => generate_nightingale_chart(data, mapping, options),
        _ => {
            return Ok(ChartResult::error(vec![format!("Unknown chart type: {}", chart_type)]));
        }
    };

    // 验证结果
    match result {
        Ok(chart_result) => {
            if chart_result.is_valid() {
                Ok(chart_result)
            } else {
                let html_len = chart_result.html.len();
                let msg = if !chart_result.warnings.is_empty() {
                    chart_result.warnings[0].clone()
                } else {
                    format!("Generated chart is invalid (html={} chars)", html_len)
                };
                Ok(ChartResult::error(vec![msg]))
            }
        }
        Err(e) => Ok(ChartResult::error(vec![format!("Chart generation error: {}", e)])),
    }
}