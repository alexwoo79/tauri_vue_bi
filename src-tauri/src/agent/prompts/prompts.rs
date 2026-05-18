// src-tauri/src/agent/prompts.rs
//
// 提示词管理模块 - 对应 Python 的 prompts.py
//
// 包含：
// - 系统提示词构建
// - 命令提示映射
// - 动态图表指南生成
// - 分析指南生成

use crate::agent::functions::charts::registry::get_chart_registry;

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
             2. Choose the most relevant numeric target_column \
                (revenue / amount / score — whatever the user mentioned, or the most business-relevant).\n\
             3. Optionally set groupby_column if the user wants a category breakdown.\n\
             4. Call run_analysis(analysis_name='Data_Decile_Analysis', sql=..., target_column=...).\n\
                SQL: SELECT <target_col>[, <groupby_col>] FROM <table>\n\
             5. Generate BOTH charts from analysis_result:\n\
                a) Bar_Chart: x=decile, y=sum — value distribution by bucket\n\
                b) Line_Chart: x=decile, y=cumulative_pct — Pareto cumulative curve\n\
             6. Conclude with a 2-4 sentence business interpretation."
        ),
        "tree" => Some(
            "The user issued the /tree command for Decision_Tree analysis.\n\
             Workflow:\n\
             1. Call get_schema ONCE.\n\
             2. target_column = the classification label column.\n\
             3. groupby_column = algorithm choice: 'ID3' | 'C4.5' | 'CART' \
                (default 'C4.5'; infer from user message if mentioned).\n\
             4. n_deciles = max_depth (0 = unlimited; default 0).\n\
             5. Call run_analysis(analysis_name='Decision_Tree', sql=..., target_column=..., \
                groupby_column=<algorithm>).\n\
                SQL: SELECT <feature_cols>, <target_col> FROM <table>\n\
             6. Generate ALL THREE charts:\n\
                a) Bar_Chart(analysis_result): x=feature, y=importance_pct — feature importance\n\
                b) Heatmap(analysis_breakdown): x=predicted, y=actual, z=count — confusion matrix\n\
                c) Line_Chart(analysis_roc): x=fpr, y=tpr, series=class — ROC curve\n\
                   Include AUC values in the chart title.\n\
             7. Conclude with a 2-4 sentence business interpretation."
        ),
        "kmeans" => Some(
            "The user issued the /kmeans command for K-Means clustering.\n\
             Workflow:\n\
             1. Call get_schema ONCE.\n\
             2. SELECT the numeric feature columns to cluster on.\n\
             3. n_deciles = K (number of clusters; default 3, or as specified by the user).\n\
             4. groupby_column = optional categorical label column for cluster purity analysis.\n\
             5. Call run_analysis(analysis_name='K_Means', sql=..., target_column=<main_numeric_col>, \
                n_deciles=<K>).\n\
                SQL: SELECT <numeric_feature_cols>[, <label_col>] FROM <table>\n\
             6. Generate ALL THREE charts:\n\
                a) Bar_Chart(analysis_result): x=cluster, y=count — cluster sizes\n\
                b) Scatter_Plot(analysis_breakdown): x=<feat1>, y=<feat2>, color=cluster\n\
                   — pick the 2 most business-relevant numeric columns for x/y\n\
                c) Line_Chart(analysis_elbow): x=k, y=inertia — elbow curve\n\
             7. A bonus table 'cluster_labels' (all original columns + cluster) is auto-created:\n\
                SELECT cluster, AVG(revenue) FROM cluster_labels GROUP BY cluster\n\
             8. Conclude with a 2-4 sentence business interpretation."
        ),
        "data" => Some(
            "The user issued the /data command to profile their data.\n\
             Call profile_data immediately as your FIRST and ONLY tool call.\n\
             Pass table_name if the user specified one; otherwise leave it empty.\n\
             Do NOT call get_schema, query_data, or any other tool first.\n\
             After profile_data returns, present the stats summary to the user — \
             the distribution charts are automatically included."
        ),
        "inset" => Some(
            "The user issued the /inset command to handle missing values.\n\
             Call clean_data(operation='fill_na', fill_method=<method>) immediately.\n\
             Determine fill_method from the user's message:\n\
               • '0' / 'zero' / '补0' → fill_method='zero'\n\
               • 'mean' / '均值' → fill_method='mean'\n\
               • 'median' / '中位数' → fill_method='median'\n\
               Default to 'mean' if the user did not specify.\n\
             Pass table_name if mentioned; otherwise leave empty (auto-detects first table).\n\
             Do NOT call any other data tools before clean_data.\n\
             After the call, tell the user the cleaned table is saved as 'cleaned_data'."
        ),
        "winsorize" => Some(
            "The user issued the /winsorize command to cap extreme values.\n\
             Call clean_data(operation='winsorize', lower_pct=<N>, upper_pct=<M>) immediately.\n\
             Extract lower_pct and upper_pct from the user's message (e.g. '1 99' → lower=1, upper=99).\n\
             Default: lower_pct=1, upper_pct=99 if not specified.\n\
             Do NOT call any other data tools before clean_data.\n\
             After the call, tell the user the result is saved as 'cleaned_data'."
        ),
        "trimming" => Some(
            "The user issued the /trimming command to remove rows outside a value range.\n\
             Call clean_data(operation='trimming', trim_column=<col>, min_val=<N>, max_val=<M>) immediately.\n\
             Extract trim_column, min_val, and max_val from the user's message.\n\
             If trim_column is unclear, call get_schema ONCE first to see numeric columns, \
             then immediately call clean_data.\n\
             Do NOT call query_data or any analysis tool.\n\
             After the call, tell the user the result is saved as 'cleaned_data'."
        ),
        "export" => Some(
            "The user issued the /export command to export data to Excel.\n\
             Call propose_excel_export — NEVER export_excel this turn.\n\
             Call propose_excel_export(tables=[\"*\"], summary=<one-line description>) immediately.\n\
             Only pass specific table names if the user explicitly asked for them.\n\
             Output NOTHING after the tool call — the UI handles confirmation."
        ),
        "excel_revise" => Some(
            "The user wants to revise the Excel export plan. \
             Current tables/filename are embedded in the user message as [CURRENT_EXCEL_JSON]. \
             Apply the requested changes and call propose_excel_export with the updated params. \
             Output NOTHING after the tool call."
        ),
        "report" => Some(
            "The user issued the /report command to generate a Word document report.\n\
             Goal: call propose_report_outline — NEVER export_report this turn.\n\n\
             Step 1 — Charts (only if user asked for charts / 带图):\n\
               If the user wants charts, generate them with generate_chart using data already\n\
               in the conversation or by running 1-2 targeted queries.\n\
               Charts are automatically bundled into the ZIP when the report is confirmed.\n\
               If the user did NOT ask for charts, skip this step entirely.\n\n\
             Step 2 — Compose the report outline from the conversation history:\n\
               title: a concise, descriptive title\n\
               sections: Executive Summary → Key Findings → Detailed Analysis → Recommendations\n\
               Each section has heading + content (plain text summary from the conversation).\n\
               Do NOT re-query or re-analyse data for the text content.\n\n\
             Step 3 — Call propose_report_outline(title=..., sections=[...]).\n\
               Output NOTHING after the tool call — the UI handles confirmation."
        ),
        "report_revise" => Some(
            "The user wants to revise the report outline. \
             Current title/sections are embedded as [CURRENT_REPORT_JSON] in the user message. \
             Apply the requested changes and call propose_report_outline with the updated params. \
             Output NOTHING after the tool call."
        ),
        "ppt" => Some(
            "The user issued /ppt. Goal: call propose_ppt_outline — NEVER generate_ppt this turn.\n\
             IMPORTANT: This MUST be done in TWO SEPARATE turns.\n\n\
             Step 1 — Charts (only if user asked for charts / 带图):\n\
               If the user wants charts in the PPT, generate them now with generate_chart.\n\
               Use data already in the conversation or run 1-2 targeted queries.\n\
               If no charts needed, skip this step.\n\n\
             Step 2 — Build the slide outline from conversation history:\n\
               title: a concise overall deck title\n\
               slides: 8-15 slide objects, each with slide_type, title, content, chart_type (if applicable)\n\
               Slide types: cover, toc, section_divider, content, chart, closing\n\
               Typical flow: Cover → TOC → Section Dividers → Content/Chart slides → Closing\n\n\
             Step 3 — Call propose_ppt_outline(title=..., slides=[...]).\n\
               Output NOTHING after the tool call — the UI handles confirmation."
        ),
        "ppt_revise" => Some(
            "The user wants to revise a PPT outline. \
             Current title/slides are embedded as [CURRENT_SLIDES_JSON] in the user message. \
             Apply the requested changes and call propose_ppt_outline with the updated slides list. \
             Output NOTHING after the tool call."
        ),
        "dashboard" => Some(
            "The user issued /dashboard. Goal: call propose_dashboard_outline — NEVER call generate_dashboard this turn.\n\
             IMPORTANT: This MUST be done in TWO SEPARATE turns.\n\n\
             Step 1 — Charts (if user asked for charts / 带图):\n\
               Generate charts with generate_chart using data in the conversation.\n\
               These will be embedded in the dashboard when confirmed.\n\n\
             Step 2 — Compose dashboard layout:\n\
               name: a concise dashboard name\n\
               widgets: list of widget objects, each with:\n\
                 - title: widget title\n\
                 - chart_type: the chart type to use\n\
                 - field_mapping: mapping of chart fields to columns\n\
                 - sql: SQL query to fetch data (or leave empty if using existing chart)\n\n\
             Step 3 — Call propose_dashboard_outline(name=..., widgets=[...]).\n\
               Output NOTHING after the tool call — the UI handles confirmation."
        ),
        "dashboard_revise" => Some(
            "The user wants to revise the dashboard outline. \
             Current name/widgets are embedded as [CURRENT_DASHBOARD_JSON] in the user message. \
             Apply the requested changes and call propose_dashboard_outline with the updated params. \
             Output NOTHING after the tool call."
        ),
        _ => None,
    }
}

/// 动态生成图表指南（对应 Python 的 _build_chart_guide）
pub fn build_chart_guide() -> String {
    let registry = get_chart_registry();
    let charts = registry.list_charts();
    
    let mut lines = Vec::new();
    let mut current_category = String::new();
    
    // 按类别分组
    let mut categories: std::collections::BTreeMap<String, Vec<&crate::agent::functions::ChartMetadata>> = 
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
    let registry = get_chart_registry();
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

/// 获取系统提示词（对应 Python 的 get_system_prompt）
pub fn get_system_prompt() -> String {
    format!(
        "你是一个数据分析专家助手。\n\
         你可以使用以下工具来帮助用户进行数据分析：\n\
         \n\
         可用图表类型：\n\
         {}\n\
         \n\
         可用分析方法：\n\
         {}\n\
         \n\
         请根据用户的问题，选择合适的工具来完成分析任务。",
        build_chart_guide(),
        build_analyze_guide()
    )
}
