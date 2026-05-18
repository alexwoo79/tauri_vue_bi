// src-tauri/src/agent/functions/registry.rs
//
// 图表注册表 - 管理所有图表类型
// 严格对标 Python Data-Analysis-Agent 的设计模式

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 图表元数据
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

    fn register(&mut self, meta: ChartMetadata) {
        self.charts.insert(meta.chart_id.clone(), meta);
    }

    fn register_all(&mut self) {
        // 对比类 COMPARING
        self.register(ChartMetadata {
            chart_id: "bar_chart".to_string(),
            name: "柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["series".to_string(), "color".to_string()],
            desc: "通过矩形高度编码数值，最常用的比较图表。建议按值降序，y轴从0开始。".to_string(),
            data_format: "x列(类别) + y列(数值)".to_string(),
            constraints: "数值列≥0，y轴从零开始".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "grouped_bar".to_string(),
            name: "分组柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "series".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "同类别多分组并排显示，便于对比。".to_string(),
            data_format: "x列(类别) + 分组列 + y列(数值)".to_string(),
            constraints: "分组数≤5".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "stacked_bar".to_string(),
            name: "堆叠柱状图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "series".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "堆叠分段比较，展示部分与整体关系。".to_string(),
            data_format: "x列(类别) + 分组列 + y列(数值)".to_string(),
            constraints: "数值≥0".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "waterfall".to_string(),
            name: "瀑布图".to_string(),
            category: "对比类 COMPARING".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["type".to_string()],
            desc: "展示从起点到终点的累积变化过程，适合分析各阶段增减贡献。".to_string(),
            data_format: "x列(阶段) + y列(数值)".to_string(),
            constraints: "支持正负值；至少2行数据".to_string(),
        });

        // 分布类 DISTRIBUTION
        self.register(ChartMetadata {
            chart_id: "histogram".to_string(),
            name: "直方图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 1,
            required_roles: vec!["value".to_string()],
            optional_roles: vec!["group".to_string()],
            desc: "展示数据分布形态，适合探索性数据分析。".to_string(),
            data_format: "value列(数值)".to_string(),
            constraints: "数值型数据".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "box_plot".to_string(),
            name: "箱线图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 1,
            required_roles: vec!["y".to_string()],
            optional_roles: vec!["group".to_string()],
            desc: "展示数据五数概括，识别异常值。".to_string(),
            data_format: "y列(数值)".to_string(),
            constraints: "数值型数据".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "violin_chart".to_string(),
            name: "小提琴图".to_string(),
            category: "分布类 DISTRIBUTION".to_string(),
            min_fields: 1,
            required_roles: vec!["y".to_string()],
            optional_roles: vec!["group".to_string()],
            desc: "结合箱线图和核密度估计，展示数据分布形状。".to_string(),
            data_format: "y列(数值)".to_string(),
            constraints: "数值型数据".to_string(),
        });

        // 关系类 RELATION
        self.register(ChartMetadata {
            chart_id: "scatter_plot".to_string(),
            name: "散点图".to_string(),
            category: "关系类 RELATION".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["size".to_string(), "color".to_string()],
            desc: "展示两个变量之间的关系，适合发现相关性。".to_string(),
            data_format: "x列(数值) + y列(数值)".to_string(),
            constraints: "数值型数据".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "bubble_plot".to_string(),
            name: "气泡图".to_string(),
            category: "关系类 RELATION".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "size".to_string()],
            optional_roles: vec!["color".to_string()],
            desc: "在散点图基础上增加尺寸编码，展示三维关系。".to_string(),
            data_format: "x列(数值) + y列(数值) + size列(数值)".to_string(),
            constraints: "数值型数据".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "heatmap".to_string(),
            name: "热力图".to_string(),
            category: "关系类 RELATION".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "通过颜色深浅展示数值大小，适合多维数据。".to_string(),
            data_format: "x列 + y列 + 数值列".to_string(),
            constraints: "数值型数据".to_string(),
        });

        // 构成类 COMPOSITION
        self.register(ChartMetadata {
            chart_id: "pie_chart".to_string(),
            name: "饼图".to_string(),
            category: "构成类 COMPOSITION".to_string(),
            min_fields: 2,
            required_roles: vec!["label".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "展示各部分占总体的比例关系。".to_string(),
            data_format: "label列 + value列".to_string(),
            constraints: "类别数≤7".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "nightingale".to_string(),
            name: "南丁格尔玫瑰图".to_string(),
            category: "构成类 COMPOSITION".to_string(),
            min_fields: 2,
            required_roles: vec!["label".to_string(), "value".to_string()],
            optional_roles: vec![],
            desc: "极坐标柱状图，展示周期性数据。".to_string(),
            data_format: "label列 + 数值列".to_string(),
            constraints: "类别数≤12".to_string(),
        });

        // 趋势类 TREND
        self.register(ChartMetadata {
            chart_id: "line_chart".to_string(),
            name: "折线图".to_string(),
            category: "趋势类 TREND".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["series".to_string()],
            desc: "展示数据随时间或连续变量的变化趋势。".to_string(),
            data_format: "x列(类别/时间) + y列(数值)".to_string(),
            constraints: "x列建议排序".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "area_chart".to_string(),
            name: "面积图".to_string(),
            category: "趋势类 TREND".to_string(),
            min_fields: 2,
            required_roles: vec!["x".to_string(), "y".to_string()],
            optional_roles: vec!["series".to_string()],
            desc: "填充折线下方区域，强调数值累积。".to_string(),
            data_format: "x列(类别/时间) + y列(数值)".to_string(),
            constraints: "数值≥0".to_string(),
        });

        self.register(ChartMetadata {
            chart_id: "stacked_area".to_string(),
            name: "堆叠面积图".to_string(),
            category: "趋势类 TREND".to_string(),
            min_fields: 3,
            required_roles: vec!["x".to_string(), "y".to_string(), "series".to_string()],
            optional_roles: vec![],
            desc: "多系列堆叠展示，展示各部分随时间的变化。".to_string(),
            data_format: "x列(时间) + y列(数值) + series列".to_string(),
            constraints: "数值≥0".to_string(),
        });

        // 流程类 FLOW
        self.register(ChartMetadata {
            chart_id: "sankey".to_string(),
            name: "桑基图".to_string(),
            category: "流程类 FLOW".to_string(),
            min_fields: 2,
            required_roles: vec!["source".to_string(), "target".to_string()],
            optional_roles: vec!["value".to_string()],
            desc: "展示流量和流向关系，适合可视化物质或信息流动。".to_string(),
            data_format: "source列 + target列 [+ value列]".to_string(),
            constraints: "分类变量".to_string(),
        });

        // 多维类 MULTIDIMENSION
        self.register(ChartMetadata {
            chart_id: "radar_chart".to_string(),
            name: "雷达图".to_string(),
            category: "多维类 MULTIDIMENSION".to_string(),
            min_fields: 1,
            required_roles: vec!["dimensions".to_string()],
            optional_roles: vec!["name".to_string()],
            desc: "多维度比较，展示对象在不同维度上的表现。".to_string(),
            data_format: "dimensions数组 + name列".to_string(),
            constraints: "维度数≤8".to_string(),
        });
    }

    pub fn get_chart(&self, chart_id: &str) -> Option<&ChartMetadata> {
        self.charts.get(chart_id)
    }

    pub fn list_charts(&self) -> Vec<&ChartMetadata> {
        self.charts.values().collect()
    }

    pub fn get_charts_by_category(&self, category: &str) -> Vec<&ChartMetadata> {
        self.charts.values()
            .filter(|c| c.category == category)
            .collect()
    }
}

lazy_static! {
    pub static ref CHART_REGISTRY: ChartRegistry = ChartRegistry::new();
}

pub fn get_chart_registry() -> &'static ChartRegistry {
    &CHART_REGISTRY
}