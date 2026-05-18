// src-tauri/src/agent/functions/charts/violin_chart.rs
//
// 小提琴图生成模块

// src-tauri/src/agent/functions/charts/violin_chart.rs
//
// 小提琴图生成模块
// Note: Plotly.rs doesn't support Violin, using BoxPlot as fallback

use anyhow::{Context, Result};
use plotly::{Plot, Layout, BoxPlot};
use plotly::common::Title;
use plotly::box_plot::BoxPoints;
use serde_json::json;
use std::collections::HashMap;

use super::base::{FieldMapping, ChartOptions, ChartResult};
use super::color_schemes::get_color_scheme;

pub fn generate_violin_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let y_col = mapping.y.context("Missing y field")?;
    let group_col = mapping.group.clone();

    let color_scheme_name = options.color_scheme.as_deref().unwrap_or("mckinsey");
    let color_scheme = get_color_scheme(color_scheme_name);

    let mut groups: HashMap<String, Vec<f64>> = HashMap::new();

    if let Some(ref group_col_name) = group_col {
        for row in &data {
            if let (Some(group), Some(y)) = (
                row.get(group_col_name).and_then(|v| v.as_str()),
                row.get(&y_col).and_then(|v| v.as_f64())
            ) {
                groups.entry(group.to_string()).or_insert_with(Vec::new).push(y);
            }
        }
    } else {
        let mut values: Vec<f64> = Vec::new();
        for row in &data {
            if let Some(y) = row.get(&y_col).and_then(|v| v.as_f64()) {
                values.push(y);
            }
        }
        groups.insert("data".to_string(), values);
    }

    if groups.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let mut plot = Plot::new();

    for (i, (group_name, values)) in groups.iter().enumerate() {
        let color = color_scheme.get_color(i).to_string();
        let y_values: Vec<Option<f64>> = values.iter().map(|v| Some(*v)).collect();
        let trace = BoxPlot::new(y_values)
            .name(group_name)
            .box_points(BoxPoints::Outliers)
            .marker(plotly::common::Marker::new().color(color));
        plot.add_trace(trace);
    }

    let layout = Layout::new()
        .title(Title::new())
        .width(options.width.unwrap_or(800) as usize)
        .height(options.height.unwrap_or(600) as usize);

    plot.set_layout(layout);

    let chart_html = plot.to_html();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <style>
        body {{ margin: 0; padding: 10px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }}
        .chart-container {{ width: 100%; height: calc(100vh - 80px); }}
        .chart-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; padding: 10px; background: #f8f9fa; border-radius: 8px; }}
        .chart-title {{ font-size: 18px; font-weight: 600; color: #333; }}
        .chart-info {{ font-size: 12px; color: #666; }}
    </style>
</head>
<body>
    <div class="chart-header">
        <div class="chart-title">{title}</div>
        <div class="chart-info">ID: violin_chart | Engine: plotly | Format: y列(数值) [+ group列]</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("小提琴图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "violin_chart",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });

    Ok(ChartResult::success(html, meta))
}