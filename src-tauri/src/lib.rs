// src-tauri/src/lib.rs
//
// 应用入口（Application Entry Point）
//
// 此文件仅负责：
//   1. 声明所有子模块
//   2. 注册 Tauri 命令处理器
//   3. 启动 Tauri 应用
//
// 所有业务逻辑已拆分到：
//   • types.rs      — 共享数据类型
//   • state.rs      — 全局状态与持久化
//   • df_util.rs    — DataFrame → ChartPayload 工具
//   • commands/     — 各领域命令（loader / chart / clean / pivot / melt / groupby / gantt / save / dataset）

// Day-1: 先打通最小 Rust Agent 闭环，暂时不编译旧的 agent 子模块。
pub mod agent; // ⚠️ 临时启用以诊断问题
               // pub mod api;     // ✅ 已移至 agent/api 下
pub mod commands;
pub mod df_util;
// pub mod llm;  // ✅ 已移至 agent/llm 下
pub mod state;
pub mod types;

use crate::commands::{
    agent_chat::{
        chart_workflow,
        chat_stream,
        clear_session_history,
        create_session,
        delete_session,
        export_excel,
        generate_chart,
        generate_dashboard,
        generate_ppt,
        generate_report,
        list_chart_types, // 新增图表生成命令
        list_sessions,
        stop_session,
    },
    chart::fetch_chart_data,
    clean::{clean_data, rollback_clean, undo_clean},
    dataset::{
        delete_datasets, get_dataset_columns, list_datasets, save_current_dataset,
        sort_and_save_dataset, switch_dataset,
    },
    datasource::{load_google_sheet_dataset, load_http_api_dataset, load_sql_dataset},
    gantt::fetch_gantt_data,
    groupby::groupby_agg,
    llm_test::{test_llm_chat, test_llm_chat_stream},
    loader::{get_dataframe_info, load_file, load_files, load_paths_as_datasets},
    melt::melt_data,
    merge::{concat_datasets, concat_paths, join_datasets},
    pivot::pivot_data,
    python_agent::{
        python_agent_health, python_agent_status, start_python_agent, stop_python_agent,
    },
    save::save_file,
    time_analysis::{
        time_agg, time_derive_columns, time_fill_missing, time_growth_rate, time_rolling_avg,
    },
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = state::load_persisted_dataset_registry() {
        eprintln!("load persisted dataset registry failed: {e}");
    }

    // Ensure OUT_DIR is set (workaround for build script issues)
    if std::env::var("OUT_DIR").is_err() {
        // Set OUT_DIR to a temp directory if not set
        let temp_dir = std::env::temp_dir().join("tauri_out_dir");
        std::fs::create_dir_all(&temp_dir).ok();
        std::env::set_var("OUT_DIR", &temp_dir);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_file,
            load_files,
            load_paths_as_datasets,
            get_dataframe_info,
            fetch_chart_data,
            pivot_data,
            melt_data,
            clean_data,
            undo_clean,
            rollback_clean,
            groupby_agg,
            fetch_gantt_data,
            save_file,
            list_datasets,
            get_dataset_columns,
            switch_dataset,
            save_current_dataset,
            delete_datasets,
            sort_and_save_dataset,
            load_sql_dataset,
            load_google_sheet_dataset,
            load_http_api_dataset,
            join_datasets,
            concat_datasets,
            concat_paths,
            time_derive_columns,
            time_agg,
            time_rolling_avg,
            time_growth_rate,
            time_fill_missing,
            // Rust Agent 命令（Day-1 最小闭环）
            create_session,
            delete_session,
            list_sessions,
            clear_session_history,
            stop_session,
            chat_stream,
            export_excel,
            generate_ppt,
            generate_report,
            generate_dashboard,
            generate_chart, // 新增：图表生成
            chart_workflow,
            list_chart_types, // 新增：列出图表类型
            start_python_agent,
            stop_python_agent,
            python_agent_status,
            python_agent_health,
            test_llm_chat,
            test_llm_chat_stream, // 新增流式命令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
