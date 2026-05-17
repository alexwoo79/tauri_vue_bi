// src-tauri/src/agent/prompts.rs
//
// 提示词管理模块 - 对应 Python 的 prompts.py
//
// 包含：
// - 系统提示词构建
// - 命令提示映射
// - 动态图表指南生成
// - 分析指南生成

use crate::agent::tools::chart_engine::ChartRegistry;

/// 命令提示映射（对应 Python 的 COMMAND_HINTS）
pub fn get_command_hint(command: &str) -> Option<&'static str> {
    match command {
        "chart" => Some(
            "The user issued the /chart command. Your primary goal for this turn is to \
             generate one or more data visualizations. Query the relevant data first, \
             then call generate_chart. End with a brief interpretation of the chart."
        ),
        "sql" => Some(
            "The user issued the /sql command. Execute the SQL they described and show \
             the results clearly formatted as a table, then provide a short insight."
        ),
        "decile" => Some(
            "The user issued the /decile command for Data_Decile_Analysis (十分位分析).\n\
             Workflow:\n\
             1. Call get_schema ONCE to understand the data.\n\
             2. Choose the most relevant numeric target_column.\n\
             3. Call run_analysis(analysis_name='Data_Decile_Analysis', sql=..., target_column=...).\n\
             4. Generate BOTH charts from analysis_result:\n\
                - Bar_Chart: x=decile, y=sum\n\
                - Line_Chart: x=decile, y=cumulative_pct\n\
             5. Conclude with a 2-4 sentence business interpretation."
        ),
        "tree" => Some(
            "The user issued the /tree command for Decision_Tree analysis.\n\
             Workflow:\n\
             1. Call get_schema ONCE.\n\
             2. target_column = the classification label column.\n\
             3. groupby_column = algorithm choice: 'ID3' | 'C4.5' | 'CART'.\n\
             4. Call run_analysis(analysis_name='Decision_Tree', ...).\n\
             5. Generate ALL THREE charts:\n\
                - Bar_Chart: feature importance\n\
                - Heatmap: confusion matrix\n\
                - Line_Chart: ROC curve\n\
             6. Conclude with a 2-4 sentence business interpretation."
        ),
        "kmeans" => Some(
            "The user issued the /kmeans command for K-Means clustering.\n\
             Workflow:\n\
             1. Call get_schema ONCE.\n\
             2. SELECT numeric feature columns.\n\
             3. n_deciles = K (number of clusters).\n\
             4. Call run_analysis(analysis_name='K_Means', ...).\n\
             5. Generate ALL THREE charts:\n\
                - Bar_Chart: cluster sizes\n\
                - Scatter_Plot: cluster view\n\
                - Line_Chart: elbow curve\n\
             6. Conclude with a 2-4 sentence business interpretation."
        ),
        "data" => Some(
            "The user issued the /data command to profile their data.\n\
             Call profile_data immediately as your FIRST and ONLY tool call."
        ),
        "inset" => Some(
            "The user issued the /inset command to handle missing values.\n\
             Call clean_data(operation='fill_na', fill_method=<method>) immediately."
        ),
        "winsorize" => Some(
            "The user issued the /winsorize command to cap extreme values.\n\
             Call clean_data(operation='winsorize', lower_pct=<N>, upper_pct=<M>) immediately."
        ),
        "trimming" => Some(
            "The user issued the /trimming command to remove rows outside a value range.\n\
             Call clean_data(operation='trimming', trim_column=<col>, min_val=<N>, max_val=<M>) immediately."
        ),
        "export" => Some(
            "The user issued the /export command to export data to Excel.\n\
             Call propose_excel_export — NEVER export_excel this turn."
        ),
        "excel_revise" => Some(
            "The user wants to revise the Excel export plan. \
             Apply the requested changes and call propose_excel_export with the updated params."
        ),
        "report" => Some(
            "The user issued the /report command to generate a Word document report.\n\
             Goal: call propose_report_outline — NEVER export_report this turn."
        ),
        "report_revise" => Some(
            "The user wants to revise the report outline. \
             Apply the requested changes and call propose_report_outline with the updated params."
        ),
        "ppt" => Some(
            "The user issued /ppt. Goal: call propose_ppt_outline — NEVER generate_ppt this turn.\n\
             IMPORTANT: This MUST be done in TWO SEPARATE turns."
        ),
        "ppt_revise" => Some(
            "The user wants to revise a PPT outline. \
             Apply the requested changes and call propose_ppt_outline with the updated slides list."
        ),
        "dashboard" => Some(
            "The user issued /dashboard. Goal: call propose_dashboard_outline — NEVER call generate_dashboard this turn.\n\
             IMPORTANT: This MUST be done in TWO SEPARATE turns."
        ),
        "dashboard_revise" => Some(
            "The user wants to revise the dashboard outline. \
             Apply the requested changes and call propose_dashboard_outline with the updated params."
        ),
        _ => None,
    }
}

/// 动态生成图表指南（对应 Python 的 _build_chart_guide）
pub fn build_chart_guide() -> String {
    let registry = ChartRegistry::new();
    let charts = registry.list_charts();
    
    let mut lines = Vec::new();
    let mut current_category = String::new();
    
    // 按类别分组
    let mut categories: std::collections::BTreeMap<String, Vec<&crate::agent::tools::chart_engine::ChartMetadata>> = 
        std::collections::BTreeMap::new();
    
    for chart in &charts {
        categories
            .entry(chart.category.clone())
            .or_insert_with(Vec::new)
            .push(chart);
    }
    
    // 生成格式化的输出
    for (category, category_charts) in &categories {
        if category != &current_category {
            current_category = category.clone();
            lines.push(format!("\n### {}", category));
        }
        
        for chart in category_charts {
            lines.push(format!(
                "- `{}` — {} ({})",
                chart.chart_id,
                chart.name,
                chart.desc
            ));
        }
    }
    
    lines.join("\n")
}

/// 获取所有图表 ID 列表（用于工具 Schema）
pub fn get_all_chart_ids() -> Vec<String> {
    let registry = ChartRegistry::new();
    registry.list_charts()
        .iter()
        .map(|c| c.chart_id.clone())
        .collect()
}

/// 构建分析指南（对应 Python 的 _build_analyze_guide）
pub fn build_analyze_guide() -> String {
    // TODO: 从注册表中动态获取分析模板
    // 目前返回静态内容
    "  Data_Decile_Analysis — 十分位分析（Decile Analysis）\n\
      Decision_Tree — 决策树分析（Decision Tree）\n\
     K_Means — K-Means 聚类分析".to_string()
}
