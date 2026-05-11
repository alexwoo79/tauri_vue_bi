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

pub mod commands;
pub mod df_util;
pub mod state;
pub mod types;

use crate::commands::{
    chart::fetch_chart_data,
    clean::{clean_data, rollback_clean, undo_clean},
    dataset::{delete_datasets, list_datasets, save_current_dataset, sort_and_save_dataset, switch_dataset},
    gantt::fetch_gantt_data,
    groupby::groupby_agg,
    loader::{get_dataframe_info, load_file, load_files, load_paths_as_datasets},
    melt::melt_data,
    merge::{concat_datasets, concat_paths, join_datasets},
    pivot::pivot_data,
    save::save_file,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = state::load_persisted_dataset_registry() {
        eprintln!("load persisted dataset registry failed: {e}");
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
            switch_dataset,
            save_current_dataset,
            delete_datasets,
            sort_and_save_dataset,
            join_datasets,
            concat_datasets,
            concat_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
