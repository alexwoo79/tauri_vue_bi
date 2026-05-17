// src-tauri/src/agent/tools/chart_tools.rs
//
// 图表生成工具 - Chart Generation Tools
//
// 提供以下功能：
// - generate_chart: 根据数据和配置生成 ECharts spec
// - 支持多种图表类型（bar, line, pie, scatter 等）
// - 支持颜色方案（mckinsey, bcg, bain, ey）
// - 自动字段映射

use anyhow::{Context, Result};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

fn is_string_dtype(dt: &DataType) -> bool {
    matches!(dt, DataType::String | DataType::Boolean)
}

fn is_numeric_dtype(dt: &DataType) -> bool {
    matches!(dt,
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 |
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 |
        DataType::Float32 | DataType::Float64
    )
}

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

/// 图表生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOptions {
    pub title: String,
    pub color_scheme: String,
    pub sort: bool,
    pub top_n: Option<usize>,
    pub orientation: Option<String>, // "vertical" or "horizontal"
}

impl Default for ChartOptions {
    fn default() -> Self {
        Self {
            title: "图表".to_string(),
            color_scheme: "mckinsey".to_string(),
            sort: true,
            top_n: None,
            orientation: Some("vertical".to_string()),
        }
    }
}

/// 图表生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResult {
    pub echarts_spec: serde_json::Value,  // ECharts JSON spec
    pub chart_type: String,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}

/// 颜色方案定义
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub palette: Vec<String>,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub positive: String,
    pub negative: String,
}

impl ColorScheme {
    /// 获取麦肯锡配色方案
    pub fn mckinsey() -> Self {
        Self {
            name: "McKinsey Blue".to_string(),
            palette: vec![
                "#003B71".to_string(),
                "#005CAB".to_string(),
                "#0083CA".to_string(),
                "#00A3E0".to_string(),
                "#7FBA00".to_string(),
                "#FFC000".to_string(),
                "#F7630C".to_string(),
                "#E2231A".to_string(),
                "#A4373A".to_string(),
                "#6B2C91".to_string(),
            ],
            primary: "#003D7A".to_string(),
            secondary: "#0084D1".to_string(),
            accent: "#00A4EF".to_string(),
            positive: "#7FBA00".to_string(),
            negative: "#DA3B01".to_string(),
        }
    }

    /// 获取 BCG 配色方案
    pub fn bcg() -> Self {
        Self {
            name: "BCG Green".to_string(),
            palette: vec![
                "#006C5B".to_string(),
                "#009879".to_string(),
                "#00B398".to_string(),
                "#CDECE5".to_string(),
                "#EAF6F3".to_string(),
                "#FFFFFF".to_string(),
            ],
            primary: "#006C5B".to_string(),
            secondary: "#009879".to_string(),
            accent: "#00B398".to_string(),
            positive: "#00B398".to_string(),
            negative: "#A6192E".to_string(),
        }
    }

    /// 获取 Bain 配色方案
    pub fn bain() -> Self {
        Self {
            name: "Bain Red".to_string(),
            palette: vec![
                "#E41E26".to_string(),
                "#FF5C5C".to_string(),
                "#A6192E".to_string(),
                "#F4E8E9".to_string(),
                "#EDEDED".to_string(),
                "#FFFFFF".to_string(),
                "#999999".to_string(),
            ],
            primary: "#E41E26".to_string(),
            secondary: "#FF5C5C".to_string(),
            accent: "#A6192E".to_string(),
            positive: "#00B398".to_string(),
            negative: "#E41E26".to_string(),
        }
    }

    /// 获取 EY 配色方案
    pub fn ey() -> Self {
        Self {
            name: "EY Yellow".to_string(),
            palette: vec![
                "#FFD100".to_string(),
                "#FFED70".to_string(),
                "#75787B".to_string(),
                "#D9D9D6".to_string(),
                "#BDBDBD".to_string(),
                "#FFFFFF".to_string(),
            ],
            primary: "#FFD100".to_string(),
            secondary: "#FFED70".to_string(),
            accent: "#75787B".to_string(),
            positive: "#7FBA00".to_string(),
            negative: "#DA3B01".to_string(),
        }
    }

    /// 根据名称获取配色方案
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "bcg" => Self::bcg(),
            "bain" => Self::bain(),
            "ey" => Self::ey(),
            _ => Self::mckinsey(), // 默认使用麦肯锡配色
        }
    }
}

/// 生成图表
pub fn tool_generate_chart(
    chart_type: &str,
    field_mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let df_guard = GLOBAL_DF.lock().unwrap();
    let df = df_guard.as_ref().context("没有加载的数据集")?;

    match chart_type {
        "bar" | "bar_chart" => generate_bar_chart(df, field_mapping, options),
        "line" | "line_chart" => generate_line_chart(df, field_mapping, options),
        "pie" | "pie_chart" => generate_pie_chart(df, field_mapping, options),
        "scatter" | "scatter_plot" => generate_scatter_chart(df, field_mapping, options),
        "area" | "area_chart" => generate_area_chart(df, field_mapping, options),
        "heatmap" | "heat_map" => generate_heatmap_chart(df, field_mapping, options),
        "boxplot" | "box_plot" | "box-and-whisker" => generate_boxplot_chart(df, field_mapping, options),
        _ => Err(anyhow::anyhow!("Unsupported chart type: {}", chart_type)),
    }
}

/// 生成柱状图
fn generate_bar_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("柱状图需要 x 字段（类别）")?;
    let y_col = mapping.y.context("柱状图需要 y 字段（数值）")?;

    // 提取数据
    let x_series = df.column(&x_col)?;
    let y_series = df.column(&y_col)?;

    // 转换为 Vec
    let x_values: Vec<String> = x_series
        .cast(&DataType::String)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let y_values: Vec<f64> = y_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    // 获取颜色方案
    let color_scheme = ColorScheme::from_name(&options.color_scheme);
    let color = color_scheme.primary;

    // 构建 ECharts spec
    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center",
            "textStyle": {
                "fontSize": 16,
                "fontWeight": "bold"
            }
        },
        "tooltip": {
            "trigger": "axis",
            "axisPointer": {
                "type": "shadow"
            }
        },
        "grid": {
            "left": "3%",
            "right": "4%",
            "bottom": "3%",
            "containLabel": true
        },
        "xAxis": {
            "type": "category",
            "data": x_values,
            "axisLabel": {
                "rotate": if x_values.len() > 10 { 45 } else { 0 },
                "interval": 0
            }
        },
        "yAxis": {
            "type": "value",
            "axisLabel": {
                "formatter": "{value}"
            }
        },
        "series": [{
            "name": y_col,
            "type": "bar",
            "data": y_values,
            "itemStyle": {
                "color": color
            },
            "emphasis": {
                "itemStyle": {
                    "shadowBlur": 10,
                    "shadowOffsetX": 0,
                    "shadowColor": "rgba(0, 0, 0, 0.5)"
                }
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "bar".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "x_col": x_col,
            "y_col": y_col,
            "n_rows": x_values.len(),
            "color_scheme": options.color_scheme,
        }),
    })
}

/// 生成折线图
fn generate_line_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("折线图需要 x 字段")?;
    let y_col = mapping.y.context("折线图需要 y 字段")?;

    // 提取数据
    let x_series = df.column(&x_col)?;
    let y_series = df.column(&y_col)?;

    let x_values: Vec<String> = x_series
        .cast(&DataType::String)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let y_values: Vec<f64> = y_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    let color_scheme = ColorScheme::from_name(&options.color_scheme);
    let color = color_scheme.primary;

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "trigger": "axis"
        },
        "xAxis": {
            "type": "category",
            "data": x_values,
            "boundaryGap": false
        },
        "yAxis": {
            "type": "value"
        },
        "series": [{
            "name": y_col,
            "type": "line",
            "data": y_values,
            "smooth": true,
            "itemStyle": {
                "color": color
            },
            "areaStyle": {
                "opacity": 0.1
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "line".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "x_col": x_col,
            "y_col": y_col,
            "n_rows": x_values.len(),
        }),
    })
}

/// 生成饼图
fn generate_pie_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let label_col = mapping.label.or(mapping.x).context("饼图需要 label 或 x 字段")?;
    let value_col = mapping.value.or(mapping.y).context("饼图需要 value 或 y 字段")?;

    let label_series = df.column(&label_col)?;
    let value_series = df.column(&value_col)?;

    let labels: Vec<String> = label_series
        .cast(&DataType::String)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let values: Vec<f64> = value_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    let color_scheme = ColorScheme::from_name(&options.color_scheme);

    // 构建饼图数据
    let data: Vec<serde_json::Value> = labels
        .iter()
        .zip(values.iter())
        .enumerate()
        .map(|(i, (label, value))| {
            serde_json::json!({
                "name": label,
                "value": value,
                "itemStyle": {
                    "color": color_scheme.palette[i % color_scheme.palette.len()]
                }
            })
        })
        .collect();

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "trigger": "item",
            "formatter": "{a} <br/>{b}: {c} ({d}%)"
        },
        "legend": {
            "orient": "vertical",
            "left": "left",
            "data": labels
        },
        "series": [{
            "name": value_col,
            "type": "pie",
            "radius": "50%",
            "data": data,
            "emphasis": {
                "itemStyle": {
                    "shadowBlur": 10,
                    "shadowOffsetX": 0,
                    "shadowColor": "rgba(0, 0, 0, 0.5)"
                }
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "pie".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "label_col": label_col,
            "value_col": value_col,
            "n_slices": labels.len(),
        }),
    })
}

/// 生成散点图
fn generate_scatter_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("散点图需要 x 字段")?;
    let y_col = mapping.y.context("散点图需要 y 字段")?;

    let x_series = df.column(&x_col)?;
    let y_series = df.column(&y_col)?;

    let x_values: Vec<f64> = x_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    let y_values: Vec<f64> = y_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    // 构建散点数据 [[x1, y1], [x2, y2], ...]
    let data: Vec<Vec<f64>> = x_values.iter().zip(y_values.iter()).map(|(x, y)| vec![*x, *y]).collect();

    let color_scheme = ColorScheme::from_name(&options.color_scheme);

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "trigger": "item"
        },
        "xAxis": {
            "type": "value",
            "name": x_col
        },
        "yAxis": {
            "type": "value",
            "name": y_col
        },
        "series": [{
            "name": "散点",
            "type": "scatter",
            "data": data,
            "symbolSize": 10,
            "itemStyle": {
                "color": color_scheme.primary
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "scatter".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "x_col": x_col,
            "y_col": y_col,
            "n_points": data.len(),
        }),
    })
}

/// 生成面积图
fn generate_area_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.context("面积图需要 x 字段")?;
    let y_col = mapping.y.context("面积图需要 y 字段")?;

    let x_series = df.column(&x_col)?;
    let y_series = df.column(&y_col)?;

    let x_values: Vec<String> = x_series
        .cast(&DataType::String)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let y_values: Vec<f64> = y_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    let color_scheme = ColorScheme::from_name(&options.color_scheme);

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "trigger": "axis"
        },
        "xAxis": {
            "type": "category",
            "data": x_values,
            "boundaryGap": false
        },
        "yAxis": {
            "type": "value"
        },
        "series": [{
            "name": y_col,
            "type": "line",
            "data": y_values,
            "smooth": true,
            "areaStyle": {
                "opacity": 0.3
            },
            "itemStyle": {
                "color": color_scheme.primary
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "area".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "x_col": x_col,
            "y_col": y_col,
            "n_rows": x_values.len(),
        }),
    })
}

/// 生成热力图
fn generate_heatmap_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    // 支持两种格式：
    // 1. 长格式：row, col, value 三列
    // 2. 宽格式：一个字符串列 + 多个数值列
    
    let row_col = mapping.x.or(mapping.group).context("热力图需要行字段（row/x/group）")?;
    let col_col = mapping.y.context("热力图需要列字段（col/y）")?;
    let val_col = mapping.value.context("热力图需要数值字段（value）")?;

    // 检查是否为宽格式（一个字符串列 + 多个数值列）
    let string_cols: Vec<String> = df
        .get_column_names()
        .iter()
        .filter(|&&col| {
            if let Ok(series) = df.column(col) {
                is_string_dtype(series.dtype())
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();

    let numeric_cols: Vec<String> = df
        .get_column_names()
        .iter()
        .filter(|&&col| {
            if let Ok(series) = df.column(col) {
                is_numeric_dtype(series.dtype())
            } else {
                false
            }
        })
        .map(|s| s.to_string())
        .collect();

    let is_wide_format = string_cols.len() == 1 && numeric_cols.len() >= 2;

    let (x_labels, y_labels, z_data) = if is_wide_format {
        // 宽格式转换
        let id_col = &string_cols[0];
        
        // 获取所有数值列（排除 ID 列）
        let value_cols: Vec<String> = numeric_cols.iter().map(|s| s.to_string()).collect();
        
        // 提取行标签
        let id_series = df.column(id_col)?;
        let y_labels: Vec<String> = id_series
            .cast(&DataType::String)?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or("").to_string())
            .collect();

        // 构建矩阵数据
            let mut z_matrix: Vec<Vec<f64>> = Vec::new();
            for i in 0..y_labels.len() {
                let mut row: Vec<f64> = Vec::new();
                for col_name in &value_cols {
                    let series = df.column(col_name)?;
                    let val = series
                        .cast(&DataType::Float64)?
                        .f64()?
                        .get(i)
                        .unwrap_or(0.0);
                    row.push(val);
                }
                z_matrix.push(row);
            }

        (value_cols, y_labels, z_matrix)
    } else {
        // 长格式：需要透视转换
        let row_series = df.column(&row_col)?;
        let col_series = df.column(&col_col)?;
        let val_series = df.column(&val_col)?;

        let rows: Vec<String> = row_series
            .cast(&DataType::String)?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or("").to_string())
            .collect();

        let cols: Vec<String> = col_series
            .cast(&DataType::String)?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or("").to_string())
            .collect();

        let vals: Vec<f64> = val_series
            .cast(&DataType::Float64)?
            .f64()?
            .into_iter()
            .map(|v| v.unwrap_or(0.0))
            .collect();

        // 去重获取唯一的 x 和 y 标签
        let mut unique_x: Vec<String> = Vec::new();
        let mut unique_y: Vec<String> = Vec::new();
        for c in &cols {
            if !unique_x.contains(c) {
                unique_x.push(c.clone());
            }
        }
        for r in &rows {
            if !unique_y.contains(r) {
                unique_y.push(r.clone());
            }
        }

        // 构建矩阵
        let mut z_matrix = vec![vec![0.0; unique_x.len()]; unique_y.len()];
        for (i, ((r, c), v)) in rows.iter().zip(cols.iter()).zip(vals.iter()).enumerate() {
            if let (Some(y_idx), Some(x_idx)) = (
                unique_y.iter().position(|y| y == r),
                unique_x.iter().position(|x| x == c),
            ) {
                z_matrix[y_idx][x_idx] = *v;
            }
        }

        (unique_x, unique_y, z_matrix)
    };

    // 构建 ECharts heatmap spec
    // 将矩阵转换为 [[x, y, value], ...] 格式
    let mut data: Vec<Vec<serde_json::Value>> = Vec::new();
    for (y_idx, row) in z_data.iter().enumerate() {
        for (x_idx, val) in row.iter().enumerate() {
            data.push(vec![
                serde_json::json!(x_idx),
                serde_json::json!(y_idx),
                serde_json::json!(val),
            ]);
        }
    }

    let color_scheme = ColorScheme::from_name(&options.color_scheme);

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "position": "top",
            "formatter": "{a0}<br/>{b0}: {c0}"
        },
        "grid": {
            "height": "70%",
            "top": "15%"
        },
        "xAxis": {
            "type": "category",
            "data": x_labels,
            "splitArea": {
                "show": true
            },
            "axisLabel": {
                "rotate": if x_labels.len() > 10 { 45 } else { 0 },
                "interval": 0
            }
        },
        "yAxis": {
            "type": "category",
            "data": y_labels,
            "splitArea": {
                "show": true
            }
        },
        "visualMap": {
            "min": 0,
            "max": z_data.iter().flat_map(|row| row.iter()).cloned().fold(f64::NEG_INFINITY, f64::max),
            "calculable": true,
            "orient": "horizontal",
            "left": "center",
            "bottom": "5%",
            "inRange": {
                "color": [
                    color_scheme.palette.get(0).unwrap_or(&"#EAF6F3".to_string()),
                    color_scheme.palette.get(2).unwrap_or(&"#00B398".to_string()),
                    color_scheme.palette.get(0).unwrap_or(&"#006C5B".to_string())
                ]
            }
        },
        "series": [{
            "name": "热力图",
            "type": "heatmap",
            "data": data,
            "label": {
                "show": true
            },
            "emphasis": {
                "itemStyle": {
                    "shadowBlur": 10,
                    "shadowColor": "rgba(0, 0, 0, 0.5)"
                }
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "heatmap".to_string(),
        warnings: if is_wide_format {
            vec![format!("自动转换宽格式数据为矩阵")]
        } else {
            vec![]
        },
        meta: serde_json::json!({
            "row_col": row_col,
            "col_col": col_col,
            "val_col": val_col,
            "n_rows": y_labels.len(),
            "n_cols": x_labels.len(),
            "format": if is_wide_format { "wide" } else { "long" },
        }),
    })
}

/// 生成箱线图
fn generate_boxplot_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let x_col = mapping.x.or(mapping.group);
    let y_col = mapping.y.context("箱线图需要 y 字段（数值）")?;

    let y_series = df.column(&y_col)?;
    let y_values: Vec<f64> = y_series
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();

    if y_values.is_empty() {
        return Err(anyhow::anyhow!("没有有效的数值数据"));
    }

    // 计算统计量：最小值、Q1、中位数、Q3、最大值
    let mut sorted = y_values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min_val = sorted.first().copied().unwrap_or(0.0);
    let max_val = sorted.last().copied().unwrap_or(0.0);
    let median = calculate_percentile(&sorted, 50.0);
    let q1 = calculate_percentile(&sorted, 25.0);
    let q3 = calculate_percentile(&sorted, 75.0);

    // 如果有分组字段，按组计算
    let data = if let Some(ref group_col) = x_col {
        let group_series = df.column(group_col)?;
        let groups: Vec<String> = group_series
            .cast(&DataType::String)?
            .str()?
            .into_iter()
            .map(|v| v.unwrap_or("").to_string())
            .collect();

        // 按组聚合
        let mut group_data: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
        for (group, val) in groups.iter().zip(y_values.iter()) {
            group_data.entry(group.clone()).or_insert_with(Vec::new).push(*val);
        }

        // 计算每组的统计量
        let mut boxplot_data: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut categories: Vec<String> = Vec::new();
        
        for (group, values) in &group_data {
            let mut sorted_vals = values.clone();
            sorted_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            
            let min_v = sorted_vals.first().copied().unwrap_or(0.0);
            let max_v = sorted_vals.last().copied().unwrap_or(0.0);
            let med = calculate_percentile(&sorted_vals, 50.0);
            let q1_v = calculate_percentile(&sorted_vals, 25.0);
            let q3_v = calculate_percentile(&sorted_vals, 75.0);

            categories.push(group.clone());
            boxplot_data.push(vec![
                serde_json::json!(min_v),
                serde_json::json!(q1_v),
                serde_json::json!(med),
                serde_json::json!(q3_v),
                serde_json::json!(max_v),
            ]);
        }

        serde_json::json!({
            "categories": categories,
            "boxplotData": boxplot_data
        })
    } else {
        // 无分组，单个箱线图
        serde_json::json!({
            "categories": [y_col],
            "boxplotData": [[
                serde_json::json!(min_val),
                serde_json::json!(q1),
                serde_json::json!(median),
                serde_json::json!(q3),
                serde_json::json!(max_val)
            ]]
        })
    };

    let color_scheme = ColorScheme::from_name(&options.color_scheme);

    let spec = serde_json::json!({
        "title": {
            "text": options.title,
            "left": "center"
        },
        "tooltip": {
            "trigger": "item",
            "axisPointer": {
                "type": "shadow"
            },
            "formatter": "{a0}<br/>{b0}: {c0}"
        },
        "grid": {
            "left": "10%",
            "right": "10%",
            "bottom": "15%"
        },
        "xAxis": {
            "type": "category",
            "data": data["categories"],
            "boundaryGap": true,
            "axisLabel": {
                "rotate": if data["categories"].as_array().map_or(0, |arr| arr.len()) > 10 { 45 } else { 0 },
                "interval": 0
            }
        },
        "yAxis": {
            "type": "value",
            "name": y_col
        },
        "series": [{
            "name": "箱线图",
            "type": "boxplot",
            "data": data["boxplotData"],
            "itemStyle": {
                "color": color_scheme.primary,
                "borderColor": color_scheme.secondary
            },
            "emphasis": {
                "itemStyle": {
                    "shadowBlur": 10,
                    "shadowOffsetX": 0,
                    "shadowColor": "rgba(0, 0, 0, 0.5)"
                }
            }
        }]
    });

    Ok(ChartResult {
        echarts_spec: spec,
        chart_type: "boxplot".to_string(),
        warnings: vec![],
        meta: serde_json::json!({
            "y_col": y_col,
            "x_col": x_col,
            "n_groups": data["categories"].as_array().map_or(0, |arr| arr.len()),
            "total_points": y_values.len(),
        }),
    })
}

/// 计算百分位数
fn calculate_percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let n = sorted_data.len();
    let index = (percentile / 100.0) * (n as f64 - 1.0);
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let weight = index - lower as f64;

    if lower == upper {
        sorted_data[lower]
    } else {
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}
