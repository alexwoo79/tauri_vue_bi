// src-tauri/src/agent/api/color_schemes.rs
//
// 配色方案 API - 对标 Python 的 api/color_schemes.py
//
// 提供：
// - 获取可用配色方案列表
// - 获取配色方案详情
// - 预览配色方案

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub id: String,
    pub name: String,
    pub description: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub colors: Vec<String>,
}

/// 内置配色方案
const BUILTIN_SCHEMES: &[(&str, &str, &str, &[&str])] = &[
    ("mckinsey", "麦肯锡蓝", "专业商务蓝色系", &["#003366", "#006699", "#3399CC", "#66CCFF", "#99CCFF"]),
    ("bcg", "波士顿绿", "经典绿色系", &["#003300", "#006600", "#339933", "#66CC66", "#99FF99"]),
    ("bain", "贝恩紫", "高端紫色系", &["#330066", "#660099", "#9933CC", "#CC66FF", "#EE99FF"]),
    ("accent_blue", "强调蓝", "现代科技蓝色", &["#1a237e", "#3949ab", "#5c6bc0", "#7986cb", "#9fa8da"]),
    ("accent_green", "强调绿", "清新绿色系", &["#1b5e20", "#43a047", "#66bb6a", "#81c784", "#a5d6a7"]),
    ("accent_orange", "强调橙", "活力橙色系", &["#bf360c", "#f4511e", "#ff7043", "#ff8a65", "#ffab91"]),
    ("accent_red", "强调红", "热情红色系", &["#b71c1c", "#e53935", "#ef5350", "#e57373", "#ef9a9a"]),
    ("gradient_blue", "渐变蓝", "现代渐变蓝色", &["#0d47a1", "#1976d2", "#42a5f5", "#90caf9", "#bbdefb"]),
    ("gradient_purple", "渐变紫", "时尚渐变紫色", &["#4a148c", "#7b1fa2", "#ab47bc", "#ce93d8", "#e1bee7"]),
];

/// 获取所有配色方案
#[tauri::command]
pub fn list_color_schemes() -> Vec<ColorScheme> {
    BUILTIN_SCHEMES.iter().map(|(id, name, desc, colors)| {
        ColorScheme {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            primary: colors[0].to_string(),
            secondary: colors[1].to_string(),
            accent: colors[2].to_string(),
            colors: colors.iter().map(|s| s.to_string()).collect(),
        }
    }).collect()
}

/// 获取单个配色方案
#[tauri::command]
pub fn get_color_scheme(scheme_id: String) -> Result<ColorScheme, String> {
    let scheme_id_clone = scheme_id.clone();
    BUILTIN_SCHEMES.iter()
        .find(|(id, _, _, _)| *id == scheme_id)
        .map(|(_, name, desc, colors)| {
            ColorScheme {
                id: scheme_id_clone,
                name: name.to_string(),
                description: desc.to_string(),
                primary: colors[0].to_string(),
                secondary: colors[1].to_string(),
                accent: colors[2].to_string(),
                colors: colors.iter().map(|s| s.to_string()).collect(),
            }
        })
        .ok_or_else(|| format!("配色方案 '{}' 不存在", scheme_id))
}

/// 验证配色方案是否有效
#[tauri::command]
pub fn validate_color_scheme(scheme_id: String) -> Result<bool, String> {
    let valid_ids: Vec<&str> = BUILTIN_SCHEMES.iter()
        .map(|(id, _, _, _)| *id)
        .collect();
    
    Ok(valid_ids.contains(&scheme_id.as_str()))
}

/// 转换 HEX 颜色到 RGB
#[tauri::command]
pub fn hex_to_rgb(hex: String) -> Result<(u8, u8, u8), String> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() != 6 {
        return Err("Invalid hex color format".to_string());
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| "Invalid red component")?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| "Invalid green component")?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| "Invalid blue component")?;
    
    Ok((r, g, b))
}

/// 转换 RGB 到 HEX 颜色
#[tauri::command]
pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}