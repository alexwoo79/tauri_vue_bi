// src-tauri/src/agent/tools/color_schemes.rs
//
// 配色方案系统（参考 Python 的 color_schemes.py）
//
// 提供多种预设配色方案：
// - McKinsey（麦肯锡风格）
// - Tableau
// - ColorBrewer
// - Material Design
// - AntV

use std::collections::HashMap;

/// 颜色定义
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub palette: Vec<String>,
    pub background: String,
    pub text: String,
}

impl ColorScheme {
    /// 获取指定索引的颜色
    pub fn get_color(&self, index: usize) -> &str {
        if self.palette.is_empty() {
            &self.primary
        } else {
            &self.palette[index % self.palette.len()]
        }
    }
}

/// 获取配色方案
pub fn get_color_scheme(name: &str) -> ColorScheme {
    match name.to_lowercase().as_str() {
        "mckinsey" | "麦肯锡" => mckinsey_scheme(),
        "tableau" => tableau_scheme(),
        "colorbrewer" | "color_brewer" => colorbrewer_scheme(),
        "material" | "material_design" => material_scheme(),
        "antv" => antv_scheme(),
        _ => mckinsey_scheme(), // 默认使用麦肯锡风格
    }
}

/// 麦肯锡风格配色（专业、稳重）
fn mckinsey_scheme() -> ColorScheme {
    ColorScheme {
        name: "McKinsey".to_string(),
        primary: "#003B71".to_string(),      // 深蓝色
        secondary: "#E4002B".to_string(),     // 红色
        palette: vec![
            "#003B71".to_string(),  // 深蓝
            "#E4002B".to_string(),  // 红
            "#FFD100".to_string(),  // 黄
            "#009CDE".to_string(),  // 浅蓝
            "#6A6A6A".to_string(),  // 灰
            "#008573".to_string(),  // 青绿
            "#F47B20".to_string(),  // 橙
            "#A50034".to_string(),  // 深红
        ],
        background: "#FFFFFF".to_string(),
        text: "#333333".to_string(),
    }
}

/// Tableau 风格配色（鲜艳、多样）
fn tableau_scheme() -> ColorScheme {
    ColorScheme {
        name: "Tableau".to_string(),
        primary: "#4E79A7".to_string(),
        secondary: "#F28E2B".to_string(),
        palette: vec![
            "#4E79A7".to_string(),  // 蓝
            "#F28E2B".to_string(),  // 橙
            "#E15759".to_string(),  // 红
            "#76B7B2".to_string(),  // 青
            "#59A14F".to_string(),  // 绿
            "#EDC948".to_string(),  // 黄
            "#B07AA1".to_string(),  // 紫
            "#FF9DA7".to_string(),  // 粉
            "#9C755F".to_string(),  // 棕
            "#BAB0AC".to_string(),  // 灰
        ],
        background: "#FFFFFF".to_string(),
        text: "#333333".to_string(),
    }
}

/// ColorBrewer 配色（适合地图和统计图表）
fn colorbrewer_scheme() -> ColorScheme {
    ColorScheme {
        name: "ColorBrewer".to_string(),
        primary: "#2B8CBE".to_string(),
        secondary: "#FD8D3C".to_string(),
        palette: vec![
            "#2B8CBE".to_string(),  // 蓝
            "#FD8D3C".to_string(),  // 橙
            "#74C476".to_string(),  // 绿
            "#E31A1C".to_string(),  // 红
            "#9970AB".to_string(),  // 紫
            "#FF7F00".to_string(),  // 橙红
            "#A6CEE3".to_string(),  // 浅蓝
            "#1F78B4".to_string(),  // 深蓝
            "#B2DF8A".to_string(),  // 浅绿
            "#33A02C".to_string(),  // 深绿
        ],
        background: "#FFFFFF".to_string(),
        text: "#333333".to_string(),
    }
}

/// Material Design 配色（现代、扁平）
fn material_scheme() -> ColorScheme {
    ColorScheme {
        name: "Material Design".to_string(),
        primary: "#2196F3".to_string(),  // Blue 500
        secondary: "#FF4081".to_string(), // Pink A200
        palette: vec![
            "#F44336".to_string(),  // Red 500
            "#E91E63".to_string(),  // Pink 500
            "#9C27B0".to_string(),  // Purple 500
            "#673AB7".to_string(),  // Deep Purple 500
            "#3F51B5".to_string(),  // Indigo 500
            "#2196F3".to_string(),  // Blue 500
            "#03A9F4".to_string(),  // Light Blue 500
            "#00BCD4".to_string(),  // Cyan 500
            "#009688".to_string(),  // Teal 500
            "#4CAF50".to_string(),  // Green 500
            "#8BC34A".to_string(),  // Light Green 500
            "#CDDC39".to_string(),  // Lime 500
            "#FFEB3B".to_string(),  // Yellow 500
            "#FFC107".to_string(),  // Amber 500
            "#FF9800".to_string(),  // Orange 500
            "#FF5722".to_string(),  // Deep Orange 500
        ],
        background: "#FAFAFA".to_string(),
        text: "#212121".to_string(),
    }
}

/// AntV 配色（蚂蚁金服可视化风格）
fn antv_scheme() -> ColorScheme {
    ColorScheme {
        name: "AntV".to_string(),
        primary: "#1890FF".to_string(),  // 蓝色
        secondary: "#F5222D".to_string(), // 红色
        palette: vec![
            "#1890FF".to_string(),  // 蓝
            "#F5222D".to_string(),  // 红
            "#FAAD14".to_string(),  // 黄
            "#52C41A".to_string(),  // 绿
            "#722ED1".to_string(),  // 紫
            "#13C2C2".to_string(),  // 青
            "#EB2F96".to_string(),  // 粉
            "#FA8C16".to_string(),  // 橙
            "#A0D911".to_string(),  // 柠檬绿
            "#2F54EB".to_string(),  // 深蓝
        ],
        background: "#FFFFFF".to_string(),
        text: "#262626".to_string(),
    }
}

/// 生成渐变色板
pub fn generate_gradient(color1: &str, color2: &str, steps: usize) -> Vec<String> {
    let c1 = hex_to_rgb(color1);
    let c2 = hex_to_rgb(color2);
    
    let mut gradient = Vec::with_capacity(steps);
    
    for i in 0..steps {
        let ratio = i as f64 / (steps - 1) as f64;
        let r = (c1.0 as f64 * (1.0 - ratio) + c2.0 as f64 * ratio) as u8;
        let g = (c1.1 as f64 * (1.0 - ratio) + c2.1 as f64 * ratio) as u8;
        let b = (c1.2 as f64 * (1.0 - ratio) + c2.2 as f64 * ratio) as u8;
        
        gradient.push(rgb_to_hex(r, g, b));
    }
    
    gradient
}

/// 十六进制颜色转 RGB
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

/// RGB 转十六进制颜色
fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}
