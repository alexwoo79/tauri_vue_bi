// src-tauri/src/commands/time_analysis.rs
//
// 时间序列分析命令（Time Series Analysis Commands）

use anyhow::{bail, Result};
use polars::prelude::*;
use std::collections::HashMap;

use crate::df_util::{df_to_payload, CHART_LIMIT};
use crate::state::{register_dataset, GLOBAL_DF};
use crate::types::{ApiResult, ChartPayload};

fn ensure_date_col(df: &DataFrame, date_col: &str) -> Result<DataFrame> {
    let dtype = df
        .column(date_col)
        .map_err(|_| anyhow::anyhow!("找不到列：{date_col}"))?
        .dtype()
        .clone();

    match dtype {
        DataType::Date => Ok(df.clone()),
        DataType::Datetime(_, _) | DataType::String => df
            .clone()
            .lazy()
            .with_column(col(date_col).cast(DataType::Date).alias(date_col))
            .collect()
            .map_err(|e| anyhow::anyhow!("列 '{}' 转换为 Date 失败：{}", date_col, e)),
        other => bail!(
            "列 '{}' 的类型为 {:?}，不支持日期操作，请先转换为 Date 类型。",
            date_col,
            other
        ),
    }
}

#[tauri::command]
pub async fn time_derive_columns(
    date_col: String,
    parts: Vec<String>,
    save_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("未加载数据，请先在[数据加载]页面导入文件。"),
            Some(df) => df.clone(),
        }
    };
    match derive_impl(&df, &date_col, &parts) {
        Ok(result_df) => {
            let name = save_name.unwrap_or_else(|| format!("日期衍生_{}", date_col));
            register_dataset(&result_df, name, "time_derive".to_string());
            *GLOBAL_DF.lock().unwrap() = Some(result_df.clone());
            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn derive_impl(df: &DataFrame, date_col: &str, parts: &[String]) -> Result<DataFrame> {
    if parts.is_empty() {
        bail!("请至少选择一个日期部分（年/月/季/周几）");
    }
    let working = ensure_date_col(df, date_col)?;

    let mut exprs: Vec<Expr> = vec![];
    for part in parts {
        let alias = format!("{date_col}_{part}");
        let expr = match part.as_str() {
            "year" => col(date_col).dt().year().alias(&alias),
            "month" => col(date_col).dt().month().cast(DataType::Int32).alias(&alias),
            "quarter" => col(date_col).dt().quarter().cast(DataType::Int32).alias(&alias),
            "weekday" => col(date_col).dt().weekday().cast(DataType::Int32).alias(&alias),
            other => bail!("未知的日期部分：{other}，支持 year/month/quarter/weekday"),
        };
        exprs.push(expr);
    }

    Ok(working.lazy().with_columns(exprs).collect()?)
}

#[tauri::command]
pub async fn time_agg(
    date_col: String,
    granularity: String,
    value_cols: Vec<String>,
    agg_func: String,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("未加载数据。"),
            Some(df) => df.clone(),
        }
    };
    match time_agg_impl(&df, &date_col, &granularity, &value_cols, &agg_func) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let default_name = format!("时间聚合_{}_{}", granularity, value_cols.join("_"));
                register_dataset(&result_df, dataset_name.unwrap_or(default_name), "time_agg".to_string());
            }
            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn time_agg_impl(
    df: &DataFrame,
    date_col: &str,
    granularity: &str,
    value_cols: &[String],
    agg_func: &str,
) -> Result<DataFrame> {
    if value_cols.is_empty() {
        bail!("至少选择一个数值列");
    }
    let working = ensure_date_col(df, date_col)?;

    let period_col = format!("_period_{granularity}");
    let period_expr = match granularity {
        "year" => col(date_col).dt().year().cast(DataType::String).alias(&period_col),
        "quarter" => {
            (col(date_col).dt().year().cast(DataType::String)
                + lit("-Q")
                + col(date_col).dt().quarter().cast(DataType::String))
            .alias(&period_col)
        }
        "month" => {
            (col(date_col).dt().year().cast(DataType::String)
                + lit("-")
                + col(date_col)
                    .dt()
                    .month()
                    .cast(DataType::String)
                    .str()
                    .zfill(lit(2u32)))
            .alias(&period_col)
        }
        "week" => {
            (col(date_col).dt().year().cast(DataType::String)
                + lit("-W")
                + col(date_col)
                    .dt()
                    .week()
                    .cast(DataType::String)
                    .str()
                    .zfill(lit(2u32)))
            .alias(&period_col)
        }
        other => bail!("未知粒度：{other}，支持 week / month / quarter / year"),
    };

    let agg_exprs: Vec<Expr> = value_cols
        .iter()
        .map(|v| {
            let e = match agg_func {
                "sum" => col(v.as_str()).sum(),
                "mean" => col(v.as_str()).mean(),
                "count" => col(v.as_str()).count().cast(DataType::Int64),
                "min" => col(v.as_str()).min(),
                "max" => col(v.as_str()).max(),
                _ => col(v.as_str()).sum(),
            };
            e.alias(v.as_str())
        })
        .collect();

    let result = working
        .lazy()
        .with_column(period_expr)
        .group_by([col(&period_col)])
        .agg(agg_exprs)
        .sort([&period_col], SortMultipleOptions::default())
        .collect()?;

    let friendly_name = match granularity {
        "year" => "年份".to_string(),
        "month" => "年月".to_string(),
        "quarter" => "年季".to_string(),
        "week" => "年周".to_string(),
        other => other.to_string(),
    };

    Ok(result.lazy().rename([&period_col], [&friendly_name], true).collect()?)
}

#[tauri::command]
pub async fn time_rolling_avg(
    date_col: String,
    value_col: String,
    window: u32,
    min_periods: u32,
    stat_func: Option<String>,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("未加载数据。"),
            Some(df) => df.clone(),
        }
    };

    let stat = stat_func.unwrap_or_else(|| "mean".to_string());
    match rolling_stat_impl(&df, &date_col, &value_col, window, min_periods, &stat) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let default_name = format!("滚动统计_{}_W{}_{}", stat, window, value_col);
                register_dataset(&result_df, dataset_name.unwrap_or(default_name), "time_rolling_avg".to_string());
            }
            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn rolling_stat_impl(
    df: &DataFrame,
    date_col: &str,
    value_col: &str,
    window: u32,
    min_periods: u32,
    stat_func: &str,
) -> Result<DataFrame> {
    if window < 1 {
        bail!("窗口大小必须 ≥ 1");
    }

    let working = ensure_date_col(df, date_col)?
        .lazy()
        .sort([date_col], SortMultipleOptions::default())
        .collect()?;

    let values: Vec<Option<f64>> = working
        .column(value_col)?
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .collect();

    let w = window as usize;
    let mp = min_periods as usize;
    let mut out: Vec<Option<f64>> = Vec::with_capacity(values.len());

    for i in 0..values.len() {
        let start = i.saturating_sub(w - 1);
        let slice: Vec<f64> = values[start..=i].iter().flatten().copied().collect();

        if slice.len() < mp {
            out.push(None);
            continue;
        }

        let val = match stat_func {
            "sum" => slice.iter().sum::<f64>(),
            "max" => slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            "min" => slice.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            "std" => {
                let mean = slice.iter().sum::<f64>() / slice.len() as f64;
                let var = slice
                    .iter()
                    .map(|v| {
                        let d = *v - mean;
                        d * d
                    })
                    .sum::<f64>()
                    / slice.len() as f64;
                var.sqrt()
            }
            _ => slice.iter().sum::<f64>() / slice.len() as f64,
        };
        out.push(Some(val));
    }

    let stat_label = match stat_func {
        "sum" => "SUM",
        "max" => "MAX",
        "min" => "MIN",
        "std" => "STD",
        _ => "MA",
    };
    let stat_col = format!("{}_W{}_{}", stat_label, window, value_col);

    let mut result = working.select([date_col, value_col])?;
    result.with_column(Series::new(stat_col.into(), out))?;
    Ok(result)
}

#[tauri::command]
pub async fn time_growth_rate(
    date_col: String,
    value_col: String,
    value_cols: Option<Vec<String>>,
    agg_func: String,
    granularity: String,
    mode: String,
    normalize_method: Option<String>,
    align_depth: Option<u32>,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("未加载数据。"),
            Some(df) => df.clone(),
        }
    };

    let target_cols = value_cols
        .unwrap_or_default()
        .into_iter()
        .filter(|c| !c.trim().is_empty())
        .collect::<Vec<String>>();
    let effective_cols = if target_cols.is_empty() {
        vec![value_col.clone()]
    } else {
        target_cols
    };
    let normalize = normalize_method.unwrap_or_else(|| "none".to_string());
    let depth = align_depth.unwrap_or(1).max(1).min(5) as usize;

    match growth_rate_impl(
        &df,
        &date_col,
        &value_col,
        &effective_cols,
        &agg_func,
        &granularity,
        &mode,
        &normalize,
        depth,
    ) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let mode_name = match mode.as_str() {
                    "yoy" => "同比",
                    "mom" => "环比",
                    "cum" => "累计",
                    "cum_yoy" => "累计同比",
                    "cum_mom" => "累计环比",
                    _ => "增长率",
                };
                let normalize_tag = match normalize.as_str() {
                    "zscore" => "_ZScore",
                    "base100" => "_Base100",
                    _ => "",
                };
                let default_name = format!(
                    "{}_{}_{}{}_D{}",
                    mode_name,
                    granularity,
                    effective_cols.join("_"),
                    normalize_tag,
                    depth
                );
                register_dataset(&result_df, dataset_name.unwrap_or(default_name), "time_growth_rate".to_string());
            }
            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn growth_rate_impl(
    df: &DataFrame,
    date_col: &str,
    primary_value_col: &str,
    value_cols: &[String],
    agg_func: &str,
    granularity: &str,
    mode: &str,
    normalize_method: &str,
    align_depth: usize,
) -> Result<DataFrame> {
    if value_cols.is_empty() {
        bail!("至少选择一个数值列");
    }

    let agg_df = time_agg_impl(df, date_col, granularity, value_cols, agg_func)?;

    let period_col = match granularity {
        "year" => "年份",
        "month" => "年月",
        "quarter" => "年季",
        "week" => "年周",
        _ => "年月",
    };

    let labels: Vec<String> = agg_df
        .column(period_col)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let shift_n: usize = match mode {
        "yoy" | "cum_yoy" => match granularity {
            "year" => 1,
            "month" => 12,
            "quarter" => 4,
            "week" => 52,
            _ => 1,
        },
        "mom" | "cum_mom" => 1,
        "cum" => 0,
        other => bail!("未知 mode：{other}，支持 yoy/mom/cum/cum_yoy/cum_mom"),
    };

    if normalize_method != "none" && normalize_method != "zscore" && normalize_method != "base100" {
        bail!("未知标准化方式：{normalize_method}，支持 none/zscore/base100");
    }

    let single_col = value_cols.len() == 1 && value_cols[0] == primary_value_col;
    let yoy_depth = if mode == "yoy" || mode == "cum_yoy" {
        align_depth.max(1)
    } else {
        1
    };
    let mut cols: Vec<Column> = vec![Series::new(period_col.into(), labels).into()];

    for value_col in value_cols {
        let vals: Vec<Option<f64>> = agg_df
            .column(value_col.as_str())?
            .cast(&DataType::Float64)?
            .f64()?
            .into_iter()
            .collect();

        let mut cum_vals: Vec<Option<f64>> = Vec::with_capacity(vals.len());
        let mut running = 0.0_f64;
        for v in &vals {
            if let Some(x) = v {
                running += *x;
                cum_vals.push(Some(running));
            } else {
                cum_vals.push(None);
            }
        }

        let cum_col_name = if single_col {
            "累计值".to_string()
        } else {
            format!("{}_累计值", value_col)
        };
        cols.push(Series::new(value_col.clone().into(), vals.clone()).into());
        cols.push(Series::new(cum_col_name.into(), cum_vals.clone()).into());

        for level in 1..=yoy_depth {
            let effective_shift = shift_n * level;
            let mut compare: Vec<Option<f64>> = vec![None; vals.len()];
            let mut growth: Vec<Option<f64>> = vec![None; vals.len()];

            if mode != "cum" && effective_shift < vals.len() {
                for i in effective_shift..vals.len() {
                    let base = match mode {
                        "cum_yoy" | "cum_mom" => cum_vals[i - effective_shift],
                        _ => vals[i - effective_shift],
                    };
                    let cur = match mode {
                        "cum_yoy" | "cum_mom" => cum_vals[i],
                        _ => vals[i],
                    };
                    compare[i] = base;
                    if let (Some(c), Some(b)) = (cur, base) {
                        if b != 0.0 {
                            growth[i] = Some(((c - b) / b * 100.0 * 100.0).round() / 100.0);
                        }
                    }
                }
            }

            let (compare_name, growth_name) = match mode {
                "yoy" => {
                    if single_col {
                        if level == 1 {
                            ("上期同比值".to_string(), "同比增长率%".to_string())
                        } else {
                            (
                                format!("前{}年同期值", level),
                                format!("对前{}年同比增长率%", level),
                            )
                        }
                    } else if level == 1 {
                        (format!("{}_上期同比值", value_col), format!("{}_同比增长率%", value_col))
                    } else {
                        (
                            format!("{}_前{}年同期值", value_col, level),
                            format!("{}_对前{}年同比增长率%", value_col, level),
                        )
                    }
                }
                "mom" => {
                    if level > 1 {
                        (String::new(), String::new())
                    } else if single_col {
                        ("上期环比值".to_string(), "环比增长率%".to_string())
                    } else {
                        (format!("{}_上期环比值", value_col), format!("{}_环比增长率%", value_col))
                    }
                }
                "cum_yoy" => {
                    if single_col {
                        if level == 1 {
                            ("累计上期同比值".to_string(), "累计同比增长率%".to_string())
                        } else {
                            (
                                format!("累计前{}年同期值", level),
                                format!("累计对前{}年同比增长率%", level),
                            )
                        }
                    } else if level == 1 {
                        (format!("{}_累计上期同比值", value_col), format!("{}_累计同比增长率%", value_col))
                    } else {
                        (
                            format!("{}_累计前{}年同期值", value_col, level),
                            format!("{}_累计对前{}年同比增长率%", value_col, level),
                        )
                    }
                }
                "cum_mom" => {
                    if level > 1 {
                        (String::new(), String::new())
                    } else if single_col {
                        ("累计上期环比值".to_string(), "累计环比增长率%".to_string())
                    } else {
                        (format!("{}_累计上期环比值", value_col), format!("{}_累计环比增长率%", value_col))
                    }
                }
                _ => (String::new(), String::new()),
            };

            if !compare_name.is_empty() {
                cols.push(Series::new(compare_name.into(), compare).into());
                cols.push(Series::new(growth_name.into(), growth).into());
            }
        }

        if normalize_method != "none" {
            let source_for_normalize = if mode == "cum" || mode == "cum_yoy" || mode == "cum_mom" {
                &cum_vals
            } else {
                &vals
            };

            let normalized = if normalize_method == "zscore" {
                let valid: Vec<f64> = source_for_normalize.iter().flatten().copied().collect();
                if valid.is_empty() {
                    vec![None; source_for_normalize.len()]
                } else {
                    let mean = valid.iter().sum::<f64>() / valid.len() as f64;
                    let var = valid
                        .iter()
                        .map(|v| {
                            let d = *v - mean;
                            d * d
                        })
                        .sum::<f64>()
                        / valid.len() as f64;
                    let std = var.sqrt();
                    source_for_normalize
                        .iter()
                        .map(|v| {
                            v.map(|x| {
                                if std == 0.0 {
                                    0.0
                                } else {
                                    ((x - mean) / std * 10000.0).round() / 10000.0
                                }
                            })
                        })
                        .collect::<Vec<Option<f64>>>()
                }
            } else {
                let base = source_for_normalize
                    .iter()
                    .flatten()
                    .copied()
                    .find(|v| *v != 0.0);
                source_for_normalize
                    .iter()
                    .map(|v| match (v, base) {
                        (Some(x), Some(b)) => Some((x / b * 100.0 * 100.0).round() / 100.0),
                        _ => None,
                    })
                    .collect::<Vec<Option<f64>>>()
            };

            let norm_name = if single_col {
                if normalize_method == "zscore" {
                    "标准化Z分数".to_string()
                } else {
                    "基期指数(100)".to_string()
                }
            } else if normalize_method == "zscore" {
                format!("{}_标准化Z分数", value_col)
            } else {
                format!("{}_基期指数(100)", value_col)
            };

            cols.push(Series::new(norm_name.into(), normalized).into());
        }
    }

    Ok(DataFrame::new(cols)?)
}

#[tauri::command]
pub async fn time_fill_missing(
    date_col: String,
    value_col: String,
    granularity: String,
    agg_func: String,
    fill_method: String,
    save_as_dataset: Option<bool>,
    dataset_name: Option<String>,
) -> ApiResult<ChartPayload> {
    let df = {
        let guard = GLOBAL_DF.lock().unwrap();
        match guard.as_ref() {
            None => return ApiResult::failure("未加载数据。"),
            Some(df) => df.clone(),
        }
    };

    match fill_missing_impl(&df, &date_col, &value_col, &granularity, &agg_func, &fill_method) {
        Ok(result_df) => {
            if save_as_dataset.unwrap_or(false) {
                let default_name = format!("时间补全_{}_{}_{}", granularity, fill_method, value_col);
                register_dataset(&result_df, dataset_name.unwrap_or(default_name), "time_fill_missing".to_string());
            }
            match df_to_payload(&result_df, Some(CHART_LIMIT)) {
                Ok(p) => ApiResult::success(p),
                Err(e) => ApiResult::failure(e.to_string()),
            }
        }
        Err(e) => ApiResult::failure(e.to_string()),
    }
}

fn parse_period_key(label: &str, granularity: &str) -> Option<i32> {
    match granularity {
        "year" => label.parse::<i32>().ok(),
        "month" => {
            let ps: Vec<&str> = label.split('-').collect();
            if ps.len() != 2 {
                return None;
            }
            let y = ps[0].parse::<i32>().ok()?;
            let m = ps[1].parse::<i32>().ok()?;
            Some(y * 12 + (m - 1))
        }
        "quarter" => {
            let ps: Vec<&str> = label.split("-Q").collect();
            if ps.len() != 2 {
                return None;
            }
            let y = ps[0].parse::<i32>().ok()?;
            let q = ps[1].parse::<i32>().ok()?;
            Some(y * 4 + (q - 1))
        }
        "week" => {
            let ps: Vec<&str> = label.split("-W").collect();
            if ps.len() != 2 {
                return None;
            }
            let y = ps[0].parse::<i32>().ok()?;
            let w = ps[1].parse::<i32>().ok()?;
            Some(y * 53 + (w - 1))
        }
        _ => None,
    }
}

fn period_label_from_key(key: i32, granularity: &str) -> String {
    match granularity {
        "year" => format!("{key}"),
        "month" => {
            let y = key.div_euclid(12);
            let m = key.rem_euclid(12) + 1;
            format!("{y}-{:02}", m)
        }
        "quarter" => {
            let y = key.div_euclid(4);
            let q = key.rem_euclid(4) + 1;
            format!("{y}-Q{q}")
        }
        "week" => {
            let y = key.div_euclid(53);
            let w = key.rem_euclid(53) + 1;
            format!("{y}-W{:02}", w)
        }
        _ => key.to_string(),
    }
}

fn fill_missing_impl(
    df: &DataFrame,
    date_col: &str,
    value_col: &str,
    granularity: &str,
    agg_func: &str,
    fill_method: &str,
) -> Result<DataFrame> {
    let agg_df = time_agg_impl(df, date_col, granularity, &[value_col.to_string()], agg_func)?;

    let period_col = match granularity {
        "year" => "年份",
        "month" => "年月",
        "quarter" => "年季",
        "week" => "年周",
        _ => "年月",
    };

    let labels: Vec<String> = agg_df
        .column(period_col)?
        .str()?
        .into_iter()
        .map(|v| v.unwrap_or("").to_string())
        .collect();

    let vals: Vec<Option<f64>> = agg_df
        .column(value_col)?
        .cast(&DataType::Float64)?
        .f64()?
        .into_iter()
        .collect();

    let mut label_to_val: HashMap<i32, Option<f64>> = HashMap::new();
    let mut keys: Vec<i32> = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        if let Some(k) = parse_period_key(label, granularity) {
            keys.push(k);
            label_to_val.insert(k, vals[i]);
        }
    }
    if keys.is_empty() {
        bail!("时间补全失败：无法解析时间周期");
    }

    let min_k = *keys.iter().min().unwrap();
    let max_k = *keys.iter().max().unwrap();
    let full_keys: Vec<i32> = (min_k..=max_k).collect();
    let full_labels: Vec<String> = full_keys
        .iter()
        .map(|k| period_label_from_key(*k, granularity))
        .collect();

    let mut out_vals: Vec<Option<f64>> = full_keys
        .iter()
        .map(|k| *label_to_val.get(k).unwrap_or(&None))
        .collect();

    let existing: Vec<f64> = out_vals.iter().flatten().copied().collect();
    let mean_val = if existing.is_empty() {
        0.0
    } else {
        existing.iter().sum::<f64>() / existing.len() as f64
    };

    match fill_method {
        "zero" => {
            for v in &mut out_vals {
                if v.is_none() {
                    *v = Some(0.0);
                }
            }
        }
        "mean" => {
            for v in &mut out_vals {
                if v.is_none() {
                    *v = Some(mean_val);
                }
            }
        }
        "ffill" => {
            let mut last: Option<f64> = None;
            for v in &mut out_vals {
                if v.is_none() {
                    *v = last;
                } else {
                    last = *v;
                }
            }
        }
        "bfill" => {
            let mut next: Option<f64> = None;
            for v in out_vals.iter_mut().rev() {
                if v.is_none() {
                    *v = next;
                } else {
                    next = *v;
                }
            }
        }
        "linear" => {
            let n = out_vals.len();
            let known_idx: Vec<usize> = out_vals
                .iter()
                .enumerate()
                .filter_map(|(i, v)| if v.is_some() { Some(i) } else { None })
                .collect();

            if !known_idx.is_empty() {
                let first = known_idx[0];
                for i in 0..first {
                    out_vals[i] = out_vals[first];
                }
                let last = *known_idx.last().unwrap();
                for i in (last + 1)..n {
                    out_vals[i] = out_vals[last];
                }
                for w in known_idx.windows(2) {
                    let i0 = w[0];
                    let i1 = w[1];
                    let v0 = out_vals[i0].unwrap_or(0.0);
                    let v1 = out_vals[i1].unwrap_or(v0);
                    let span = (i1 - i0) as f64;
                    for k in (i0 + 1)..i1 {
                        let t = (k - i0) as f64 / span;
                        out_vals[k] = Some(v0 + (v1 - v0) * t);
                    }
                }
            }
        }
        other => bail!("未知填充方式：{other}，支持 zero/mean/linear/ffill/bfill"),
    }

    let cols: Vec<Column> = vec![
        Series::new(period_col.into(), full_labels).into(),
        Series::new(value_col.into(), out_vals).into(),
    ];
    Ok(DataFrame::new(cols)?)
}
