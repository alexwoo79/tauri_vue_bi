// src-tauri/src/agent/tools_schema.rs
//
// LLM 工具 JSON Schema 定义 - 对应 Python 的 tools_schema.py
//
// 定义所有可供 LLM 调用的工具的 JSON Schema
// 这些 Schema 会被传递给 LLM API，让模型知道可以调用哪些工具

use serde_json::{json, Value};

/// 获取所有 Agent 工具的 JSON Schema 列表
pub fn get_agent_tools() -> Vec<Value> {
    vec![
        // get_schema
        json!({
            "type": "function",
            "function": {
                "name": "get_schema",
                "description": "Get the full schema of the connected data source — tables, columns, types, and row counts. Always call this first when the user asks about data you haven't seen yet.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        }),
        
        // create_analysis_table
        json!({
            "type": "function",
            "function": {
                "name": "create_analysis_table",
                "description": "Create a summary analysis table from the dataset",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "group_by": {
                            "type": "string",
                            "description": "Column name to group by"
                        }
                    },
                    "required": []
                }
            }
        }),
        
        // generate_chart
        json!({
            "type": "function",
            "function": {
                "name": "generate_chart",
                "description": "Generate a chart from the dataset",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "chart_type": {
                            "type": "string",
                            "description": "Type of chart: bar, line, pie, scatter, histogram, box, heatmap, area, stacked_bar, stacked_area, radar, sankey, waterfall, bubble, nightingale"
                        },
                        "title": {
                            "type": "string",
                            "description": "Chart title"
                        },
                        "x_column": {
                            "type": "string",
                            "description": "Column name for X axis"
                        },
                        "y_column": {
                            "type": "string",
                            "description": "Column name for Y axis"
                        },
                        "group_column": {
                            "type": "string",
                            "description": "Column name for grouping (optional)"
                        },
                        "color_scheme": {
                            "type": "string",
                            "description": "Color scheme name (mckinsey, bcg, bain, etc.)"
                        }
                    },
                    "required": ["chart_type"]
                }
            }
        }),
        
        // export_report
        json!({
            "type": "function",
            "function": {
                "name": "export_report",
                "description": "Export analysis report to file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "description": "Export format: excel, ppt, word, pdf"
                        },
                        "sections": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "content": {"type": "string"},
                                    "chart_ids": {"type": "array", "items": {"type": "string"}}
                                }
                            },
                            "description": "Report sections"
                        }
                    },
                    "required": ["format"]
                }
            }
        }),
        
        // data_profile
        json!({
            "type": "function",
            "function": {
                "name": "data_profile",
                "description": "Generate data profile and statistics",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "columns": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Specific columns to analyze (optional)"
                        }
                    },
                    "required": []
                }
            }
        }),
        
        // handle_missing
        json!({
            "type": "function",
            "function": {
                "name": "handle_missing",
                "description": "Handle missing values in the dataset",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "strategy": {
                            "type": "string",
                            "description": "Strategy: drop, mean, median, mode, forward_fill, backward_fill"
                        },
                        "columns": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Columns to apply the strategy (optional)"
                        }
                    },
                    "required": ["strategy"]
                }
            }
        }),
        
        // winsorize
        json!({
            "type": "function",
            "function": {
                "name": "winsorize",
                "description": "Apply winsorization to reduce outliers",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "lower_quantile": {
                            "type": "number",
                            "description": "Lower quantile (0-1)",
                            "default": 0.05
                        },
                        "upper_quantile": {
                            "type": "number",
                            "description": "Upper quantile (0-1)",
                            "default": 0.95
                        },
                        "columns": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Columns to apply winsorization (optional)"
                        }
                    },
                    "required": []
                }
            }
        }),
        
        // decile_analysis
        json!({
            "type": "function",
            "function": {
                "name": "decile_analysis",
                "description": "Perform decile analysis on a numeric column",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "column": {
                            "type": "string",
                            "description": "Column name to analyze"
                        }
                    },
                    "required": ["column"]
                }
            }
        }),
        
        // kmeans_clustering
        json!({
            "type": "function",
            "function": {
                "name": "kmeans_clustering",
                "description": "Perform K-Means clustering",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "columns": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Columns to use for clustering"
                        },
                        "k": {
                            "type": "integer",
                            "description": "Number of clusters",
                            "default": 5
                        }
                    },
                    "required": ["columns"]
                }
            }
        }),
        
        // decision_tree
        json!({
            "type": "function",
            "function": {
                "name": "decision_tree",
                "description": "Train a decision tree classifier",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "target_column": {
                            "type": "string",
                            "description": "Target column for classification"
                        },
                        "feature_columns": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Feature columns"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum tree depth",
                            "default": 5
                        }
                    },
                    "required": ["target_column", "feature_columns"]
                }
            }
        })
    ]
}

/// 获取图表生成工具 Schema（简化版）
pub fn get_chart_tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "generate_chart",
            "description": "Generate a chart from the dataset",
            "parameters": {
                "type": "object",
                "properties": {
                    "chart_type": {
                        "type": "string",
                        "description": "Type of chart: bar, line, pie, scatter, histogram, box, heatmap, area, stacked_bar, stacked_area, radar, sankey, waterfall, bubble, nightingale"
                    },
                    "title": {"type": "string", "description": "Chart title"},
                    "x_column": {"type": "string", "description": "X axis column"},
                    "y_column": {"type": "string", "description": "Y axis column"},
                    "group_column": {"type": "string", "description": "Group column (optional)"},
                    "color_scheme": {"type": "string", "description": "Color scheme name"}
                },
                "required": ["chart_type"]
            }
        }
    })
}