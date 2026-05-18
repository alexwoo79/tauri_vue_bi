// src-tauri/src/agent/functions/charts/sankey_chart.rs
//
// 桑基图生成模块
// Note: Plotly.rs Sankey API may differ, using HTML-based approach

use anyhow::{Context, Result};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use super::base::{FieldMapping, ChartOptions, ChartResult};

pub fn generate_sankey_chart(
    data: Vec<HashMap<String, serde_json::Value>>,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    let source_col = mapping.source.context("Missing source field")?;
    let target_col = mapping.target.context("Missing target field")?;
    let value_col = mapping.value.clone();

    let mut all_nodes: HashSet<String> = HashSet::new();
    let mut sources: Vec<String> = Vec::new();
    let mut targets: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();

    for row in &data {
        if let (Some(source), Some(target)) = (
            row.get(&source_col).and_then(|v| v.as_str()),
            row.get(&target_col).and_then(|v| v.as_str())
        ) {
            let source_str = source.to_string();
            let target_str = target.to_string();
            
            all_nodes.insert(source_str.clone());
            all_nodes.insert(target_str.clone());
            sources.push(source_str);
            targets.push(target_str);
            
            if let Some(ref val_col) = value_col {
                if let Some(value) = row.get(val_col).and_then(|v| v.as_f64()) {
                    values.push(value);
                } else {
                    values.push(1.0);
                }
            } else {
                values.push(1.0);
            }
        }
    }

    if all_nodes.is_empty() {
        return Ok(ChartResult::error(vec!["No valid data".to_string()]));
    }

    let node_list: Vec<String> = all_nodes.into_iter().collect();
    let node_map: HashMap<String, usize> = node_list.iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    let source_indices: Vec<usize> = sources.iter()
        .map(|s| *node_map.get(s).unwrap())
        .collect();
    let target_indices: Vec<usize> = targets.iter()
        .map(|t| *node_map.get(t).unwrap())
        .collect();

    // 使用自定义 HTML 渲染桑基图
    let node_labels_json = serde_json::to_string(&node_list).unwrap_or_default();
    let source_indices_json = serde_json::to_string(&source_indices).unwrap_or_default();
    let target_indices_json = serde_json::to_string(&target_indices).unwrap_or_default();
    let values_json = serde_json::to_string(&values).unwrap_or_default();

    let chart_html = format!(
        r#"
<script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
<div id="sankey-chart" style="width: 100%; height: 100%;"></div>
<script>
    var data = {{
        type: 'sankey',
        node: {{
            pad: 15,
            thickness: 20,
            line: {{ color: 'black', width: 0.5 }},
            label: {node_labels}
        }},
        link: {{
            source: {source_indices},
            target: {target_indices},
            value: {values}
        }}
    }};
    var layout = {{
        title: '',
        font: {{ size: 12 }}
    }};
    Plotly.newPlot('sankey-chart', [data], layout);
</script>
        "#,
        node_labels = node_labels_json,
        source_indices = source_indices_json,
        target_indices = target_indices_json,
        values = values_json
    );

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
        <div class="chart-info">ID: sankey_chart | Engine: plotly | Format: source列 + target列 [+ value列]</div>
    </div>
    <div class="chart-container">{chart_html}</div>
</body>
</html>"#,
        title = options.title.clone().unwrap_or("桑基图".to_string()),
        chart_html = chart_html
    );

    let meta = json!({
        "chart_id": "sankey_chart",
        "n_rows": data.len(),
        "n_nodes": node_list.len(),
    });

    Ok(ChartResult::success(html, meta))
}
