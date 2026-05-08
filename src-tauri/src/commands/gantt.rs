// src-tauri/src/commands/gantt.rs
//
// 甘特图数据命令（Gantt Chart Data Command）

use crate::df_util::df_to_payload;
use crate::state::GLOBAL_DF;
use crate::types::{ApiResult, ChartPayload};

#[tauri::command]
pub async fn fetch_gantt_data(
    task_col: String,
    start_col: String,
    end_col: String,
    project_col: Option<String>,
    color_col: Option<String>,
    milestone_col: Option<String>,
    detail_col: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => {
                return ApiResult::failure(
                    "No data loaded. Please select a file and click Load.",
                )
            }
            Some(df) => df.clone(),
        }
    };

    let mut keep_cols: Vec<String> = vec![task_col.clone(), start_col.clone(), end_col.clone()];
    if let Some(ref c) = project_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = color_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = milestone_col {
        keep_cols.push(c.clone());
    }
    if let Some(ref c) = detail_col {
        keep_cols.push(c.clone());
    }

    let column_names = df.get_column_names();
    let valid: Vec<&str> = keep_cols
        .iter()
        .filter(|c| column_names.iter().any(|n| n.as_str() == c.as_str()))
        .map(|c| c.as_str())
        .collect();

    match df.select(valid) {
        Ok(result_df) => match df_to_payload(&result_df, None) {
            Ok(p) => ApiResult::success(p),
            Err(e) => ApiResult::failure(e.to_string()),
        },
        Err(e) => ApiResult::failure(e.to_string()),
    }
}
