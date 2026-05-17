# 图表生成工具使用指南

## 概述

`chart_tools` 模块提供了在 Rust 中生成 ECharts 图表规格（spec）的功能。它支持 7 种基础图表类型，并可以根据数据自动映射字段。

## 支持的图表类型

| 图表类型 | 工具名称 | 描述 | 必需字段 |
|---------|---------|------|---------|
| 柱状图 | `bar` / `bar_chart` | 比较不同类别的数值 | x (类别), y (数值) |
| 折线图 | `line` / `line_chart` | 显示趋势变化 | x (类别/时间), y (数值) |
| 饼图 | `pie` / `pie_chart` | 显示占比关系 | label (标签), value (数值) |
| 散点图 | `scatter` / `scatter_plot` | 显示两个变量的关系 | x (数值), y (数值) |
| 面积图 | `area` / `area_chart` | 显示累积趋势 | x (类别/时间), y (数值) |
| **热力图** | `heatmap` / `heat_map` | **矩阵数据可视化** | **row/x, col/y, value** |
| **箱线图** | `boxplot` / `box_plot` | **数据分布和异常值检测** | **y (数值), x (可选分组)** |

## 颜色方案

支持 4 种企业级配色方案：

| 方案名称 | 主色 | 描述 |
|---------|------|------|
| `mckinsey` | #003D7A | 麦肯锡蓝色系（默认） |
| `bcg` | #006C5B | BCG 绿色系 |
| `bain` | #E41E26 | Bain 红色系 |
| `ey` | #FFD100 | EY 黄色系 |

## API 使用

### 1. 通过 LLM 工具调用

LLM 可以调用 `generate_chart` 工具来生成图表：

```json
{
  "name": "generate_chart",
  "arguments": {
    "chart_type": "bar",
    "title": "各地区销售对比",
    "x": "region",
    "y": "sales",
    "color_scheme": "mckinsey",
    "sort": true,
    "top_n": 10
  }
}
```

### 2. 直接调用函数

```rust
use crate::agent::tools::chart_tools;

let mapping = chart_tools::FieldMapping {
    x: Some("region".to_string()),
    y: Some("sales".to_string()),
    ..Default::default()
};

let options = chart_tools::ChartOptions {
    title: "各地区销售对比".to_string(),
    color_scheme: "mckinsey".to_string(),
    sort: true,
    top_n: Some(10),
    orientation: Some("vertical".to_string()),
};

let result = chart_tools::tool_generate_chart("bar", mapping, options)?;

// 获取 ECharts spec
let echarts_spec = result.echarts_spec;
println!("{}", serde_json::to_string_pretty(&echarts_spec)?);
```

## 返回结果

`ChartResult` 结构包含：

```rust
pub struct ChartResult {
    pub echarts_spec: serde_json::Value,  // ECharts JSON spec
    pub chart_type: String,                // 图表类型
    pub warnings: Vec<String>,             // 警告信息
    pub meta: serde_json::Value,           // 元数据
}
```

### ECharts Spec 示例（柱状图）

```json
{
  "title": {
    "text": "各地区销售对比",
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
    "data": ["东部", "西部", "北部", "南部"],
    "axisLabel": {
      "rotate": 0,
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
    "name": "sales",
    "type": "bar",
    "data": [150000, 120000, 180000, 95000],
    "itemStyle": {
      "color": "#003D7A"
    },
    "emphasis": {
      "itemStyle": {
        "shadowBlur": 10,
        "shadowOffsetX": 0,
        "shadowColor": "rgba(0, 0, 0, 0.5)"
      }
    }
  }]
}
```

### ECharts Spec 示例（热力图）

```json
{
  "title": {
    "text": "产品销售热力图",
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
    "data": ["Q1", "Q2", "Q3", "Q4"],
    "splitArea": {
      "show": true
    }
  },
  "yAxis": {
    "type": "category",
    "data": ["产品A", "产品B", "产品C"],
    "splitArea": {
      "show": true
    }
  },
  "visualMap": {
    "min": 0,
    "max": 1000,
    "calculable": true,
    "orient": "horizontal",
    "left": "center",
    "bottom": "5%",
    "inRange": {
      "color": ["#EAF6F3", "#00B398", "#006C5B"]
    }
  },
  "series": [{
    "name": "热力图",
    "type": "heatmap",
    "data": [
      [0, 0, 150], [1, 0, 200], [2, 0, 180], [3, 0, 220],
      [0, 1, 300], [1, 1, 350], [2, 1, 320], [3, 1, 380]
    ],
    "label": {
      "show": true
    }
  }]
}
```

### ECharts Spec 示例（箱线图）

```json
{
  "title": {
    "text": "销售额分布分析",
    "left": "center"
  },
  "tooltip": {
    "trigger": "item",
    "axisPointer": {
      "type": "shadow"
    }
  },
  "xAxis": {
    "type": "category",
    "data": ["东部", "西部", "北部", "南部"]
  },
  "yAxis": {
    "type": "value",
    "name": "销售额"
  },
  "series": [{
    "name": "箱线图",
    "type": "boxplot",
    "data": [
      [50000, 80000, 120000, 150000, 200000],
      [40000, 70000, 100000, 130000, 180000],
      [60000, 90000, 140000, 170000, 220000],
      [30000, 50000, 80000, 110000, 150000]
    ]
  }]
}
```

## 前端集成

前端可以使用 `vue-echarts` 组件渲染 ECharts spec：

```vue
<template>
  <v-chart :option="chartSpec" autoresize />
</template>

<script setup>
import { ref } from 'vue'
import VChart from 'vue-echarts'

const chartSpec = ref(null)

// 从后端获取 ECharts spec
async function loadChart() {
  const response = await invoke('chat_stream', {
    sessionId: 'xxx',
    message: '生成一个销售柱状图',
    // ...
  })
  
  // 解析响应中的 echarts_spec
  chartSpec.value = JSON.parse(response.echarts_spec)
}
</script>
```

## 字段映射规则

### 柱状图 (bar)
- **x**: 类别字段（字符串）
- **y**: 数值字段（数字）
- **series**: 可选，分组字段

### 折线图 (line)
- **x**: 类别或时间字段
- **y**: 数值字段
- **series**: 可选，多条线时使用

### 饼图 (pie)
- **label** 或 **x**: 标签字段
- **value** 或 **y**: 数值字段

### 散点图 (scatter)
- **x**: X 轴数值
- **y**: Y 轴数值
- **size**: 可选，点的大小
- **color**: 可选，点的颜色

### 面积图 (area)
- **x**: 类别或时间字段
- **y**: 数值字段

### 热力图 (heatmap)
- **row** 或 **x** 或 **group**: 行标签
- **col** 或 **y**: 列标签
- **value**: 数值
- **支持宽格式自动转换**：一个字符串列 + 多个数值列

### 箱线图 (boxplot)
- **y**: 数值字段（必需）
- **x** 或 **group**: 可选，分组字段
- **自动计算统计量**：最小值、Q1、中位数、Q3、最大值

## 选项配置

`ChartOptions` 结构：

```rust
pub struct ChartOptions {
    pub title: String,              // 图表标题
    pub color_scheme: String,       // 颜色方案
    pub sort: bool,                 // 是否排序
    pub top_n: Option<usize>,       // 只显示前 N 条
    pub orientation: Option<String>, // "vertical" 或 "horizontal"
}
```

## 扩展现有图表

要添加新的图表类型，需要：

1. 在 `chart_tools.rs` 中实现生成函数：

```rust
fn generate_new_chart(
    df: &DataFrame,
    mapping: FieldMapping,
    options: ChartOptions,
) -> Result<ChartResult> {
    // 提取数据
    // 构建 ECharts spec
    // 返回 ChartResult
}
```

2. 在 `tool_generate_chart()` 中添加匹配分支：

```rust
match chart_type {
    // ... 现有图表 ...
    "new_chart" => generate_new_chart(df, mapping, options),
    _ => Err(anyhow::anyhow!("Unsupported chart type: {}", chart_type)),
}
```

## 注意事项

1. **数据类型**: 确保 x/y 字段的数据类型正确（字符串或数值）
2. **空值处理**: 工具会自动过滤空值
3. **性能**: 大数据集建议设置 `top_n` 限制
4. **颜色循环**: 饼图等会使用颜色方案中的所有颜色循环
5. **热力图格式**: 支持长格式和宽格式，自动检测并转换
6. **箱线图统计**: 自动计算五数概括（最小值、Q1、中位数、Q3、最大值）

## 未来扩展

计划支持的图表类型：

- [ ] 堆叠柱状图 (stacked_bar)
- [ ] 分组柱状图 (grouped_bar)
- [x] 热力图 (heatmap) ✅
- [x] 箱线图 (boxplot) ✅
- [ ] 直方图 (histogram)
- [ ] 瀑布图 (waterfall)
- [ ] 雷达图 (radar)
- [ ] 漏斗图 (funnel)
- [ ] 桑基图 (sankey)
- [ ] 树图 (treemap)

---

**作者**: alex  
**日期**: 2026-05-16  
**版本**: 1.1 (新增热力图和箱线图)
