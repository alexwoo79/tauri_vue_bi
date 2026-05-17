// src-tauri/src/agent/tools/chart_engine.rs
//
// Chart Engine - 基于 Plotly.rs 的图表生成引擎
//
// 设计原则：
// 1. 完全参考 Python Agent 的图表类型和参数
// 2. 使用 Plotly.rs 替代 Plotly.py
// 3. 保持与 Python 相同的 API 结构（mapping, options）
// 4. 支持 43 种图表类型（逐步实现）
//
// 架构：
// - ChartRegistry: 图表注册表（类似 Python 的 REGISTRY）
// - ChartGenerator: 图表生成器 trait
// - 各图表类型的独立实现模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout};
use plotly::common::Title;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 字段映射（与 Python 保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub x: Option<String>,
    pub y: Option<String>,
    pub series: Option<String>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub label: Option<String>,
    pub value: Option<String>,
    pub group: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub dimensions: Option<Vec<String>>,
    pub parents: Option<String>,
    pub labels: Option<String>,
    pub values: Option<String>,
    pub order: Option<String>,
}

impl FieldMapping {
    /// 从 HashMap 创建 FieldMapping
    pub fn from_hashmap(map: &HashMap<String, serde_json::Value>) -> Self {
        Self {
            x: map.get("x").and_then(|v| v.as_str()).map(|s| s.to_string()),
            y: map.get("y").and_then(|v| v.as_str()).map(|s| s.to_string()),
            series: map.get("series").and_then(|v| v.as_str()).map(|s| s.to_string()),
            color: map.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
            size: map.get("size").and_then(|v| v.as_str()).map(|s| s.to_string()),
            label: map.get("label").and_then(|v| v.as_str()).map(|s| s.to_string()),
            value: map.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()),
            group: map.get("group").and_then(|v| v.as_str()).map(|s| s.to_string()),
            source: map.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
            target: map.get("target").and_then(|v| v.as_str()).map(|s| s.to_string()),
            dimensions: map.get("dimensions").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()
            }),
            parents: map.get("parents").and_then(|v| v.as_str()).map(|s| s.to_string()),
            labels: map.get("labels").and_then(|v| v.as_str()).map(|s| s.to_string()),
            values: map.get("values").and_then(|v| v.as_str()).map(|s| s.to_string()),
            order: map.get("order").and_then(|v| v.as_str()).map(|s| s.to_string()),
        }
    }
}

/// 图表选项（与 Python 保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOptions {
    pub title: Option<String>,
    pub color_scheme: Option<String>,
    pub orientation: Option<String>,  // "v" or "h"
    pub sort: Option<bool>,
    pub top_n: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Default for ChartOptions {
    fn default() -> Self {
        Self {
            title: Some("Chart".to_string()),
            color_scheme: Some("mckinsey".to_string()),
            orientation: Some("v".to_string()),
            sort: Some(true),
            top_n: None,
            width: Some(800),
            height: Some(600),
        }
    }
}

/// 图表生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResult {
    pub html: String,
    pub chart_type: String,
    pub warnings: Vec<String>,
    pub meta: serde_json::Value,
}

impl ChartResult {
    pub fn error(msg: &str) -> Self {
        Self {
            html: String::new(),
            chart_type: String::new(),
            warnings: vec![msg.to_string()],
            meta: serde_json::json!({}),
        }
    }
    
    pub fn success(html: String, chart_type: &str, meta: serde_json::Value) -> Self {
        Self {
            html,
            chart_type: chart_type.to_string(),
            warnings: vec![],
            meta,
        }
    }
}

/// 图表元数据（类似 Python 的 ChartMetadata）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartMetadata {
    pub chart_id: String,
    pub name: String,
    pub category: String,
    pub min_fields: usize,
    pub required_roles: Vec<String>,
    pub optional_roles: Vec<String>,
    pub desc: String,
    pub data_format: String,
    pub constraints: String,
}

/// 图表注册表
pub struct ChartRegistry {
    charts: HashMap<String, ChartMetadata>,
}

impl ChartRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            charts: HashMap::new(),
        };
        
        registry.register_all();
        
        registry
    }
    
    fn register_all(&mut self) {
        self.register(ChartMetadata {
            chart_id: "Bar_Chart".to_string(),
            name: "柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["series".to_string(), "color".to_string()],
            desc: "通过矩形高度编码数值，最常用的比较图表".to_string(),
            data_format: "x列(类别) + y列(数值)".to_string(),
            constraints: "数值列≥0，y轴从零开始".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Grouped_Bar_Chart".to_string(),
            name: "分组柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "series".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "同类别多分组并排显示，便于对比".to_string(),
            data_format: "x列(类别) + 分组列 + y列(数值)".to_string(),
            constraints: "分组数≤5".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Stacked_Bar_Chart".to_string(),
            name: "堆叠柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "series".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "堆叠分段比较，展示部分与整体关系".to_string(),
            data_format: "x列(类别) + 分组列 + y列(数值)".to_string(),
            constraints: "数值≥0".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Waterfall".to_string(),
            name: "瀑布图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["type".to_string()],
            desc: "展示从起点到终点的累积变化过程，适合分析各阶段增减贡献".to_string(),
            data_format: "x列(阶段) + y列(数值)".to_string(),
            constraints: "支持正负值；至少2行数据".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Heatmap".to_string(),
            name: "热力图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "通过颜色深浅展示数值大小，适合多维数据".to_string(),
            data_format: "x列 + y列 + 数值列".to_string(),
            constraints: "支持大量数据点".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Sankey_Chart".to_string(),
            name: "桑基图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["source".to_string(), "target".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "展示流向和流量".to_string(),
            data_format: "源 + 目标 + 流量".to_string(),
            constraints: "适合流程展示".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Line_Chart".to_string(),
            name: "折线图".to_string(),
            category: "时间趋势类 TIME".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["series".to_string()],
            desc: "展示数据随时间或其他连续变量的变化趋势".to_string(),
            data_format: "x列(时间/连续) + y列(数值)".to_string(),
            constraints: "适合时间序列".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Histogram_Pareto_chart".to_string(),
            name: "直方图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 1,
            required_roles: vec!["value".to_string()],
            optional_roles: vec![],
            desc: "展示数据分布情况".to_string(),
            data_format: "数值列".to_string(),
            constraints: "需要足够的数据点".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Violin_Chart".to_string(),
            name: "小提琴图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec![],
            desc: "结合箱线图和核密度估计，展示数据分布".to_string(),
            data_format: "类别列 + 数值列".to_string(),
            constraints: "适合对比多个分布".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Box-and-Whisker_Plot".to_string(),
            name: "箱线图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec![],
            desc: "展示数据的五数概括".to_string(),
            data_format: "类别列 + 数值列".to_string(),
            constraints: "适合检测异常值".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Pie_Chart".to_string(),
            name: "饼图".to_string(),
            category: "占比类 PART-TO-WHOLE".to_string(),
            min_fields: 2,
            required_roles: vec!["label".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "展示各部分占整体的比例".to_string(),
            data_format: "标签列 + 数值列".to_string(),
            constraints: "适合分类较少的场景".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Treemap".to_string(),
            name: "树状图".to_string(),
            category: "占比类 PART-TO-WHOLE".to_string(),
            min_fields: 2,
            required_roles: vec!["labels".to_string(), "values".to_string()],
            optional_roles: vec!["parents".to_string()],
            desc: "通过矩形面积展示层级数据的占比关系".to_string(),
            data_format: "标签列 + 数值列 + 父节点列（可选）".to_string(),
            constraints: "适合展示层级结构".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Sunburst_Diagram".to_string(),
            name: "旭日图".to_string(),
            category: "占比类 PART-TO-WHOLE".to_string(),
            min_fields: 2,
            required_roles: vec!["labels".to_string(), "values".to_string()],
            optional_roles: vec!["parents".to_string()],
            desc: "多层环形图，展示层级数据的占比".to_string(),
            data_format: "标签列 + 数值列 + 父节点列".to_string(),
            constraints: "适合展示多层级结构".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Nightingale_Chart".to_string(),
            name: "南丁格尔玫瑰图".to_string(),
            category: "占比类 PART-TO-WHOLE".to_string(),
            min_fields: 2,
            required_roles: vec!["label".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "极坐标柱状图，展示周期性数据".to_string(),
            data_format: "标签列 + 数值列".to_string(),
            constraints: "适合时间周期数据".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Bubble_Plot".to_string(),
            name: "气泡图".to_string(),
            category: "关系类 RELATIONSHIP".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "size".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "通过气泡大小展示第三维度数据".to_string(),
            data_format: "x列 + y列 + 大小列".to_string(),
            constraints: "适合四维数据展示".to_string(),
        });
        
        self.register(ChartMetadata {
            chart_id: "Radar_Chart".to_string(),
            name: "雷达图".to_string(),
            category: "关系类 RELATIONSHIP".to_string(),
            min_fields: 2,
            required_roles: vec!["label".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "多维数据对比，展示各项指标的均衡性".to_string(),
            data_format: "标签列 + 数值列".to_string(),
            constraints: "适合多维度评估".to_string(),
        });
    }
    
    fn register(&mut self, metadata: ChartMetadata) {
        self.charts.insert(metadata.chart_id.clone(), metadata);
    }
    
    pub fn get_chart(&self, chart_id: &str) -> Option<&ChartMetadata> {
        self.charts.get(chart_id)
    }
    
    pub fn list_charts(&self) -> Vec<&ChartMetadata> {
        self.charts.values().collect()
    }
    
    pub fn list_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.charts.values()
            .map(|c| c.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }
}

impl Default for ChartRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成图表的主函数（类似 Python 的 generate_chart）
pub fn generate_chart(
    chart_type: &str,
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let registry = ChartRegistry::new();
    let metadata = registry.get_chart(chart_type)
        .context(format!("Chart type '{}' not found", chart_type))?;
    
    let mapping = if is_mapping_empty(&mapping) {
        auto_detect_mapping(&data, chart_type)
    } else {
        mapping
    };
    
    for role in &metadata.required_roles {
        match role.as_str() {
            "x" if mapping.x.is_none() => {
                return Ok(ChartResult::error("Missing required field: x"));
            }
            "y" if mapping.y.is_none() => {
                return Ok(ChartResult::error("Missing required field: y"));
            }
            "series" if mapping.series.is_none() => {
                return Ok(ChartResult::error("Missing required field: series"));
            }
            "source" if mapping.source.is_none() => {
                return Ok(ChartResult::error("Missing required field: source"));
            }
            "target" if mapping.target.is_none() => {
                return Ok(ChartResult::error("Missing required field: target"));
            }
            "value" if mapping.value.is_none() => {
                return Ok(ChartResult::error("Missing required field: value"));
            }
            "label" if mapping.label.is_none() => {
                return Ok(ChartResult::error("Missing required field: label"));
            }
            "labels" if mapping.labels.is_none() => {
                return Ok(ChartResult::error("Missing required field: labels"));
            }
            "values" if mapping.values.is_none() => {
                return Ok(ChartResult::error("Missing required field: values"));
            }
            "size" if mapping.size.is_none() => {
                return Ok(ChartResult::error("Missing required field: size"));
            }
            _ => {}
        }
    }
    
    match chart_type {
        "Bar_Chart" => generate_bar_chart(data, mapping, options),
        "Grouped_Bar_Chart" => generate_grouped_bar_chart(data, mapping, options),
        "Stacked_Bar_Chart" => generate_stacked_bar_chart(data, mapping, options),
        "Line_Chart" => generate_line_chart(data, mapping, options),
        "Heatmap" => generate_heatmap(data, mapping, options),
        "Violin_Chart" => generate_violin_chart(data, mapping, options),
        "Box-and-Whisker_Plot" => generate_box_plot(data, mapping, options),
        "Histogram_Pareto_chart" => generate_histogram(data, mapping, options),
        "Waterfall" => generate_waterfall(data, mapping, options),
        "Sankey_Chart" => generate_sankey_chart(data, mapping, options),
        "Bubble_Plot" => generate_bubble_plot(data, mapping, options),
        "Pie_Chart" => generate_pie_chart(data, mapping, options),
        "Radar_Chart" => generate_radar_chart(data, mapping, options),
        "Nightingale_Chart" => generate_nightingale_chart(data, mapping, options),
        "Treemap" | "Sunburst_Diagram" => Ok(ChartResult::error("Chart type not implemented in plotly 0.14")),
        _ => Ok(ChartResult::error(&format!("Chart type '{}' not yet implemented", chart_type))),
    }
}

/// 检查 mapping 是否为空
fn is_mapping_empty(mapping: &FieldMapping) -> bool {
    mapping.x.is_none()
        && mapping.y.is_none()
        && mapping.series.is_none()
        && mapping.color.is_none()
        && mapping.size.is_none()
        && mapping.label.is_none()
        && mapping.value.is_none()
        && mapping.group.is_none()
        && mapping.source.is_none()
        && mapping.target.is_none()
        && mapping.dimensions.is_none()
        && mapping.parents.is_none()
        && mapping.labels.is_none()
        && mapping.values.is_none()
        && mapping.order.is_none()
}

/// 自动检测字段映射（参考 Python 的 _auto_detect_mapping）
fn auto_detect_mapping(
    data: &[HashMap<String, serde_json::Value>],
    chart_type: &str,
) -> FieldMapping {
    if data.is_empty() {
        return FieldMapping {
            x: None, y: None, series: None, color: None, size: None,
            label: None, value: None, group: None, source: None, target: None,
            dimensions: None, parents: None, labels: None, values: None, order: None,
        };
    }
    
    let first_row = &data[0];
    let mut string_cols: Vec<String> = Vec::new();
    let mut numeric_cols: Vec<String> = Vec::new();
    
    for (key, value) in first_row {
        if value.is_string() {
            string_cols.push(key.clone());
        } else if value.is_number() {
            numeric_cols.push(key.clone());
        }
    }
    
    let mut mapping = FieldMapping {
        x: None, y: None, series: None, color: None, size: None,
        label: None, value: None, group: None, source: None, target: None,
        dimensions: None, parents: None, labels: None, values: None, order: None,
    };
    
    match chart_type {
        "Bar_Chart" | "Line_Chart" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            } else if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
            }
        }
        "Grouped_Bar_Chart" | "Stacked_Bar_Chart" => {
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
        "Scatter_Plot" | "Bubble_Plot" => {
            if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
                if numeric_cols.len() >= 3 {
                    mapping.size = Some(numeric_cols[2].clone());
                }
            }
        }
        "Pie_Chart" | "Nightingale_Chart" | "Radar_Chart" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.label = Some(string_cols[0].clone());
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        "Heatmap" => {
            if numeric_cols.len() >= 2 {
                mapping.x = Some(numeric_cols[0].clone());
                mapping.y = Some(numeric_cols[1].clone());
                if numeric_cols.len() >= 3 {
                    mapping.value = Some(numeric_cols[2].clone());
                }
            }
        }
        "Histogram_Pareto_chart" => {
            if !numeric_cols.is_empty() {
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        "Violin_Chart" | "Box-and-Whisker_Plot" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            } else if !numeric_cols.is_empty() {
                mapping.y = Some(numeric_cols[0].clone());
            }
        }
        "Waterfall" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            }
        }
        "Sankey_Chart" => {
            if string_cols.len() >= 2 && !numeric_cols.is_empty() {
                mapping.source = Some(string_cols[0].clone());
                mapping.target = Some(string_cols[1].clone());
                mapping.value = Some(numeric_cols[0].clone());
            }
        }
        "Treemap" | "Sunburst_Diagram" => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.labels = Some(string_cols[0].clone());
                mapping.values = Some(numeric_cols[0].clone());
                if string_cols.len() >= 2 {
                    mapping.parents = Some(string_cols[1].clone());
                }
            }
        }
        _ => {
            if !string_cols.is_empty() && !numeric_cols.is_empty() {
                mapping.x = Some(string_cols[0].clone());
                mapping.y = Some(numeric_cols[0].clone());
            }
        }
    }
    
    mapping
}

// ────────────────────────────────────────────────────────────
// 图表生成函数实现（参考 Python 代码）
// ────────────────────────────────────────────────────────────

/// 生成柱状图
fn generate_bar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::{Bar, Layout};
    use crate::agent::tools::color_schemes::get_color_scheme;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    
    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);
    let color = &color_scheme.primary;
    
    let mut x_values: Vec<String> = Vec::new();
    let mut y_values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(x), Some(y)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            x_values.push(x.to_string());
            y_values.push(y);
        }
    }
    
    if x_values.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    if options.sort.unwrap_or(true) {
        let mut paired: Vec<(String, f64)> = x_values.into_iter()
            .zip(y_values.into_iter())
            .collect();
        
        paired.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        if options.orientation.as_deref() == Some("v") {
            paired.reverse();
        }
        
        x_values = paired.iter().map(|(x, _)| x.clone()).collect();
        y_values = paired.iter().map(|(_, y)| *y).collect();
    }
    
    if let Some(top_n) = options.top_n {
        x_values.truncate(top_n);
        y_values.truncate(top_n);
    }
    
    let trace = Bar::new(x_values, y_values)
        .name(&options.title.clone().unwrap_or("Bar Chart".to_string()));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Bar_Chart",
        "n_rows": data.len(),
        "x_col": x_col,
        "y_col": y_col,
        "color_scheme": color_scheme.name,
    });
    
    Ok(ChartResult::success(html, "Bar_Chart", meta))
}

/// 生成分组柱状图
fn generate_grouped_bar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::Bar;
    use crate::agent::tools::color_schemes::get_color_scheme;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    let series_col = mapping.series.context("Missing series field")?;
    
    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);
    
    let mut groups: HashMap<String, (Vec<String>, Vec<f64>)> = HashMap::new();
    
    for row in &data {
        if let (Some(x), Some(y), Some(series)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64()),
            row.get(&series_col).and_then(|v| v.as_str())
        ) {
            let entry = groups.entry(series.to_string()).or_insert_with(|| (vec![], vec![]));
            entry.0.push(x.to_string());
            entry.1.push(y);
        }
    }
    
    if groups.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let mut plot = Plot::new();
    
    for (i, (series_name, (x_vals, y_vals))) in groups.iter().enumerate() {
        let color = color_scheme.palette.get(i % color_scheme.palette.len()).cloned().unwrap_or("#003B71".to_string());
        let trace = Bar::new(x_vals.clone(), y_vals.clone())
            .name(series_name)
            .marker(plotly::common::Marker::new().color(color));
        plot.add_trace(trace);
    }
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Grouped_Bar_Chart",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });
    
    Ok(ChartResult::success(html, "Grouped_Bar_Chart", meta))
}

/// 生成堆叠柱状图
fn generate_stacked_bar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::Bar;
    use crate::agent::tools::color_schemes::get_color_scheme;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    let series_col = mapping.series.context("Missing series field")?;
    
    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);
    
    let mut groups: HashMap<String, (Vec<String>, Vec<f64>)> = HashMap::new();
    
    for row in &data {
        if let (Some(x), Some(y), Some(series)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64()),
            row.get(&series_col).and_then(|v| v.as_str())
        ) {
            let entry = groups.entry(series.to_string()).or_insert_with(|| (vec![], vec![]));
            entry.0.push(x.to_string());
            entry.1.push(y);
        }
    }
    
    if groups.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let mut plot = Plot::new();
    
    for (i, (series_name, (x_vals, y_vals))) in groups.iter().enumerate() {
        let color = color_scheme.palette.get(i % color_scheme.palette.len()).cloned().unwrap_or("#003B71".to_string());
        let trace = Bar::new(x_vals.clone(), y_vals.clone())
            .name(series_name)
            .marker(plotly::common::Marker::new().color(color));
        plot.add_trace(trace);
    }
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Stacked_Bar_Chart",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });
    
    Ok(ChartResult::success(html, "Stacked_Bar_Chart", meta))
}

/// 生成折线图
fn generate_line_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::Scatter;
    use plotly::common::Mode;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    
    let mut x_values: Vec<String> = Vec::new();
    let mut y_values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(x), Some(y)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            x_values.push(x.to_string());
            y_values.push(y);
        }
    }
    
    if x_values.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let trace = Scatter::new(x_values, y_values)
        .mode(Mode::LinesMarkers)
        .name(&options.title.clone().unwrap_or("Line Chart".to_string()));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Line_Chart",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Line_Chart", meta))
}

/// 生成热力图
fn generate_heatmap(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::HeatMap;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut x_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut y_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut matrix: HashMap<(String, String), f64> = HashMap::new();
    
    for row in &data {
        if let (Some(x), Some(y), Some(value)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            x_set.insert(x.to_string());
            y_set.insert(y.to_string());
            matrix.insert((x.to_string(), y.to_string()), value);
        }
    }
    
    let x_values: Vec<String> = x_set.into_iter().collect();
    let y_values: Vec<String> = y_set.into_iter().collect();
    
    let mut z: Vec<Vec<f64>> = Vec::new();
    for y in &y_values {
        let mut row: Vec<f64> = Vec::new();
        for x in &x_values {
            row.push(*matrix.get(&(x.clone(), y.clone())).unwrap_or(&0.0));
        }
        z.push(row);
    }
    
    let x_dim = x_values.len();
    let y_dim = y_values.len();
    let trace = HeatMap::new(x_values, y_values, z);
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Heatmap",
        "n_rows": data.len(),
        "x_dim": x_dim,
        "y_dim": y_dim,
    });

    Ok(ChartResult::success(html, "Heatmap", meta))
}

/// 生成小提琴图
fn generate_violin_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::BoxPlot;
    use plotly::box_plot::BoxPoints;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    
    let mut groups: HashMap<String, Vec<f64>> = HashMap::new();
    
    for row in &data {
        if let (Some(x), Some(y)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            groups.entry(x.to_string()).or_insert_with(Vec::new).push(y);
        }
    }
    
    if groups.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let mut plot = Plot::new();
    
    for (group_name, values) in &groups {
        let y_values: Vec<Option<f64>> = values.iter().map(|v| Some(*v)).collect();
        let trace = BoxPlot::new(y_values)
            .name(group_name)
            .box_points(BoxPoints::All);
        plot.add_trace(trace);
    }
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Violin_Chart",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });
    
    Ok(ChartResult::success(html, "Violin_Chart", meta))
}

/// 生成箱线图
fn generate_box_plot(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::BoxPlot;
    use plotly::box_plot::BoxPoints;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    
    let mut groups: HashMap<String, Vec<f64>> = HashMap::new();
    
    for row in &data {
        if let (Some(x), Some(y)) = (
            row.get(&x_col).and_then(|v| v.as_str()),
            row.get(&y_col).and_then(|v| v.as_f64())
        ) {
            groups.entry(x.to_string()).or_insert_with(Vec::new).push(y);
        }
    }
    
    if groups.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let mut plot = Plot::new();
    
    for (group_name, values) in &groups {
        let y_values: Vec<Option<f64>> = values.iter().map(|v| Some(*v)).collect();
        let trace = BoxPlot::new(y_values)
            .name(group_name)
            .box_points(BoxPoints::Outliers);
        plot.add_trace(trace);
    }
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Box-and-Whisker_Plot",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });
    
    Ok(ChartResult::success(html, "Box-and-Whisker_Plot", meta))
}

/// 生成直方图
fn generate_histogram(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::histogram::Histogram;
    
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let Some(value) = row.get(&value_col).and_then(|v| v.as_f64()) {
            values.push(value);
        }
    }
    
    if values.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let trace = Histogram::new(values);
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Histogram_Pareto_chart",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Histogram_Pareto_chart", meta))
}

/// 生成瀑布图
fn generate_waterfall(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    Err(anyhow::anyhow!("Waterfall chart not yet implemented for plotly 0.14"))
}

/// 生成桑基图
fn generate_sankey_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::sankey::{Sankey, Link, Node};
    
    let source_col = mapping.source.context("Missing source field")?;
    let target_col = mapping.target.context("Missing target field")?;
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut node_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut sources: Vec<usize> = Vec::new();
    let mut targets: Vec<usize> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(source), Some(target), Some(value)) = (
            row.get(&source_col).and_then(|v| v.as_str()),
            row.get(&target_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            node_set.insert(source.to_string());
            node_set.insert(target.to_string());
        }
    }
    
    let node_list: Vec<String> = node_set.into_iter().collect();
    let node_index: HashMap<String, usize> = node_list.iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();
    
    for row in &data {
        if let (Some(source), Some(target), Some(value)) = (
            row.get(&source_col).and_then(|v| v.as_str()),
            row.get(&target_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            if let (Some(&src_idx), Some(&tgt_idx)) = (
                node_index.get(source),
                node_index.get(target)
            ) {
                sources.push(src_idx);
                targets.push(tgt_idx);
                values.push(value);
            }
        }
    }
    
    let node = Node::new()
        .label(node_list.iter().map(|s| s.as_str()).collect::<Vec<&str>>());
    
    let link = Link::new()
        .source(sources)
        .target(targets)
        .value(values);
    
    let sankey = Sankey::new()
        .node(node)
        .link(link);
    
    let mut plot = Plot::new();
    plot.add_trace(sankey);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(1000) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Sankey_Chart",
        "n_rows": data.len(),
        "n_nodes": node_index.len(),
    });
    
    Ok(ChartResult::success(html, "Sankey_Chart", meta))
}

/// 生成气泡图
fn generate_bubble_plot(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::Scatter;
    use plotly::common::Mode;
    
    let x_col = mapping.x.context("Missing x field")?;
    let y_col = mapping.y.context("Missing y field")?;
    let size_col = mapping.size.context("Missing size field")?;
    
    let mut x_values: Vec<f64> = Vec::new();
    let mut y_values: Vec<f64> = Vec::new();
    let mut sizes: Vec<f64> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    
    for row in &data {
        if let (Some(x), Some(y), Some(size)) = (
            row.get(&x_col).and_then(|v| v.as_f64()),
            row.get(&y_col).and_then(|v| v.as_f64()),
            row.get(&size_col).and_then(|v| v.as_f64())
        ) {
            x_values.push(x);
            y_values.push(y);
            sizes.push(size);
            
            let label = if let Some(text) = row.get("label").and_then(|v| v.as_str()) {
                text.to_string()
            } else {
                format!("({}, {})", x, y)
            };
            labels.push(label);
        }
    }
    
    if x_values.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let trace = Scatter::new(x_values, y_values)
        .mode(Mode::Markers)
        .name(&options.title.clone().unwrap_or("Bubble Plot".to_string()))
        .marker(
            plotly::common::Marker::new()
                .opacity(0.6)
        );
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Bubble_Plot",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Bubble_Plot", meta))
}

/// 生成饼图
fn generate_pie_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::Pie;
    
    let label_col = mapping.label.context("Missing label field")?;
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut labels: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(label), Some(value)) = (
            row.get(&label_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            labels.push(label.to_string());
            values.push(value);
        }
    }
    
    if labels.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let trace = Pie::new(values)
        .labels(labels)
        .name(&options.title.clone().unwrap_or("Pie Chart".to_string()));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Pie_Chart",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Pie_Chart", meta))
}

/// 生成雷达图
fn generate_radar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::Scatter;
    use plotly::common::Mode;
    
    let label_col = mapping.label.context("Missing label field")?;
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut labels: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(label), Some(value)) = (
            row.get(&label_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            labels.push(label.to_string());
            values.push(value);
        }
    }
    
    if labels.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let n = labels.len();
    let angles: Vec<f64> = (0..n)
        .map(|i| 2.0 * std::f64::consts::PI * i as f64 / n as f64)
        .collect();
    
    let x_values: Vec<f64> = angles.iter()
        .zip(values.iter())
        .map(|(&angle, &value)| value * angle.cos())
        .collect();
    
    let y_values: Vec<f64> = angles.iter()
        .zip(values.iter())
        .map(|(&angle, &value)| value * angle.sin())
        .collect();
    
    let mut x_closed = x_values.clone();
    let mut y_closed = y_values.clone();
    let mut labels_closed = labels.clone();
    
    x_closed.push(x_values[0]);
    y_closed.push(y_values[0]);
    labels_closed.push(labels[0].clone());
    
    let trace = Scatter::new(x_closed, y_closed)
        .mode(Mode::LinesMarkers)
        .name(&options.title.clone().unwrap_or("Radar Chart".to_string()));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Radar_Chart",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Radar_Chart", meta))
}

/// 生成南丁格尔玫瑰图
fn generate_nightingale_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    use plotly::traces::Bar;
    
    let label_col = mapping.label.context("Missing label field")?;
    let value_col = mapping.value.context("Missing value field")?;
    
    let mut labels: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    
    for row in &data {
        if let (Some(label), Some(value)) = (
            row.get(&label_col).and_then(|v| v.as_str()),
            row.get(&value_col).and_then(|v| v.as_f64())
        ) {
            labels.push(label.to_string());
            values.push(value);
        }
    }
    
    if labels.is_empty() {
        return Ok(ChartResult::error("No valid data"));
    }
    
    let trace = Bar::new(labels.clone(), values.clone())
        .name(&options.title.clone().unwrap_or("Nightingale Rose Chart".to_string()));
    
    let mut plot = Plot::new();
    plot.add_trace(trace);
    
    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);
    
    plot.set_layout(layout);
    
    let html = plot.to_html();
    
    let meta = serde_json::json!({
        "chart_id": "Nightingale_Chart",
        "n_rows": data.len(),
    });
    
    Ok(ChartResult::success(html, "Nightingale_Chart", meta))
}