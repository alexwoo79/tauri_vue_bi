// src-tauri/src/agent/functions/charts/stacked_bar_chart.rs
//
// 堆叠柱状图生成模块

use anyhow::{Context, Result};
use plotly::{Plot, Layout, Bar};
use plotly::common::Title;
use serde_json::json;
use std::collections::HashMap;

use super::base::{FieldMapping, ChartOptions, ChartResult};
use super::color_schemes::get_color_scheme;

pub fn generate_stacked_bar_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
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
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let mut plot = Plot::new();

    for (i, (series_name, (x_vals, y_vals))) in groups.iter().enumerate() {
        let color = color_scheme.get_color(i).to_string();
        let trace = Bar::new(x_vals.clone(), y_vals.clone())
            .name(series_name)
            .marker(plotly::common::Marker::new().color(color));
        plot.add_trace(trace);
    }

    let layout = Layout::new()
        .title(Title::new())
        .bar_mode(plotly::layout::BarMode::Stack)
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
        <div class="chart-info">ID: stacked_bar | Engine: plotly | Format: x列(类别) + 分组列 + y列(数值)</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("堆叠柱状图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "stacked_bar",
        "n_rows": data.len(),
        "n_groups": groups.len(),
    });

    Ok(ChartResult::success(html, meta))
}