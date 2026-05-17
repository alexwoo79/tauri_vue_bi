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
                "description": "Extract specific fields from the raw data and materialise the result as a new queryable table. Use this to: (1) select only the columns needed for the current analysis, (2) pre-aggregate or filter large datasets before charting, (3) join / reshape data into the exact shape a chart requires.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sql": {
                            "type": "string",
                            "description": "SQL SELECT that defines the analysis table"
                        },
                        "table_name": {
                            "type": "string",
                            "description": "Name for the new temp table (default: 'analysis_data')"
                        }
                    },
                    "required": ["sql"]
                }
            }
        }),
        
        // query_data
        json!({
            "type": "function",
            "function": {
                "name": "query_data",
                "description": "Execute a SQL SELECT query and return the results as a table.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sql": {
                            "type": "string",
                            "description": "A valid SQL SELECT statement using actual column/table names from the schema."
                        }
                    },
                    "required": ["sql"]
                }
            }
        }),
        
        // run_analysis
        json!({
            "type": "function",
            "function": {
                "name": "run_analysis",
                "description": "Run a built-in statistical analysis template on the data. Steps: (1) call get_schema to know the tables/columns, (2) call run_analysis with the appropriate parameters, (3) generate charts from the result tables.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "analysis_name": {
                            "type": "string",
                            "description": "Analysis template name: 'Data_Decile_Analysis', 'Decision_Tree', or 'K_Means'"
                        },
                        "sql": {
                            "type": "string",
                            "description": "SQL SELECT to fetch the data for analysis"
                        },
                        "target_column": {
                            "type": "string",
                            "description": "The target/label column for the analysis"
                        },
                        "groupby_column": {
                            "type": "string",
                            "description": "Optional grouping column or algorithm parameter"
                        },
                        "n_deciles": {
                            "type": "integer",
                            "description": "Number of deciles/clusters (for K-Means: K value)"
                        }
                    },
                    "required": ["analysis_name", "sql", "target_column"]
                }
            }
        }),
        
        // profile_data
        json!({
            "type": "function",
            "function": {
                "name": "profile_data",
                "description": "Profile the data to show statistics like count, mean, std, min, max, null counts, etc. Call this when the user wants to understand their data distribution.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "table_name": {
                            "type": "string",
                            "description": "Table name to profile (leave empty for first table)"
                        }
                    },
                    "required": []
                }
            }
        }),
        
        // clean_data
        json!({
            "type": "function",
            "function": {
                "name": "clean_data",
                "description": "Clean the data by handling missing values, capping extremes, or trimming rows. Supports operations: fill_na, winsorize, trimming.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "operation": {
                            "type": "string",
                            "description": "Operation type: 'fill_na', 'winsorize', or 'trimming'"
                        },
                        "fill_method": {
                            "type": "string",
                            "description": "For fill_na: 'mean', 'median', 'zero', etc."
                        },
                        "lower_pct": {
                            "type": "number",
                            "description": "For winsorize: lower percentile (e.g., 1)"
                        },
                        "upper_pct": {
                            "type": "number",
                            "description": "For winsorize: upper percentile (e.g., 99)"
                        },
                        "trim_column": {
                            "type": "string",
                            "description": "For trimming: column to trim on"
                        },
                        "min_val": {
                            "type": "number",
                            "description": "For trimming: minimum value"
                        },
                        "max_val": {
                            "type": "number",
                            "description": "For trimming: maximum value"
                        },
                        "table_name": {
                            "type": "string",
                            "description": "Table name to clean (leave empty for first table)"
                        }
                    },
                    "required": ["operation"]
                }
            }
        }),
        
        // generate_chart
        json!({
            "type": "function",
            "function": {
                "name": "generate_chart",
                "description": "Generate a data visualization chart. First query the data, then call this with the appropriate chart_type and field_mapping.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "chart_type": {
                            "type": "string",
                            "description": "Chart type ID (e.g., 'Bar_Chart', 'Line_Chart', 'Pie_Chart', etc.)"
                        },
                        "field_mapping": {
                            "type": "object",
                            "description": "Mapping of data columns to chart roles (x, y, series, label, value, etc.)"
                        },
                        "options": {
                            "type": "object",
                            "description": "Optional chart configuration (title, colors, etc.)"
                        }
                    },
                    "required": ["chart_type", "field_mapping"]
                }
            }
        }),
        
        // propose_excel_export
        json!({
            "type": "function",
            "function": {
                "name": "propose_excel_export",
                "description": "Propose an Excel export plan. Call this when the user issues /export command. The UI will show a confirmation card.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "tables": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of table names to export (use ['*'] for all tables)"
                        },
                        "filename": {
                            "type": "string",
                            "description": "Optional custom filename"
                        },
                        "summary": {
                            "type": "string",
                            "description": "One-line description of the export content"
                        }
                    },
                    "required": ["tables"]
                }
            }
        }),
        
        // propose_ppt_outline
        json!({
            "type": "function",
            "function": {
                "name": "propose_ppt_outline",
                "description": "Propose a PowerPoint presentation outline. Call this after gathering data in a separate turn. NEVER call this in the same turn as data queries.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Presentation title"
                        },
                        "slides": {
                            "type": "array",
                            "items": {"type": "object"},
                            "description": "Array of slide objects (8-15 slides). Each slide has layout, title, content, and optional chart data."
                        }
                    },
                    "required": ["title", "slides"]
                }
            }
        }),
        
        // propose_report_outline
        json!({
            "type": "function",
            "function": {
                "name": "propose_report_outline",
                "description": "Propose a Word report outline. Compose the outline from conversation history.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Report title"
                        },
                        "sections": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "heading": {"type": "string"},
                                    "content": {"type": "string"}
                                }
                            },
                            "description": "Report sections: Executive Summary → Key Findings → Detailed Analysis → Recommendations"
                        }
                    },
                    "required": ["title", "sections"]
                }
            }
        }),
        
        // propose_dashboard_outline
        json!({
            "type": "function",
            "function": {
                "name": "propose_dashboard_outline",
                "description": "Propose a dashboard layout with widgets. Call this after gathering data in a separate turn.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Dashboard name"
                        },
                        "widgets": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "chart_type": {"type": "string"},
                                    "sql": {"type": "string"},
                                    "position": {
                                        "type": "object",
                                        "properties": {
                                            "x": {"type": "integer"},
                                            "y": {"type": "integer"},
                                            "w": {"type": "integer"},
                                            "h": {"type": "integer"}
                                        }
                                    }
                                }
                            },
                            "description": "Array of widget objects (2-6 widgets). Total width should be 12 units."
                        }
                    },
                    "required": ["name", "widgets"]
                }
            }
        }),
    ]
}
