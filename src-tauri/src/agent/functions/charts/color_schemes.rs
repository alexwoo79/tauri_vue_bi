// src-tauri/src/agent/functions/color_schemes.rs
//
// 配色方案系统 - 企业级配色方案
// 支持: McKinsey, BCG, Bain, EY 等咨询公司配色

use lazy_static::lazy_static;
use std::collections::HashMap;

/// 配色方案定义
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub background: String,
    pub text: String,
    pub colors: Vec<String>,
}

impl ColorScheme {
    /// 获取颜色列表
    pub fn get_colors_list(&self, count: Option<usize>) -> Vec<String> {
        let n = count.unwrap_or(self.colors.len());
        if n <= self.colors.len() {
            self.colors[0..n].to_vec()
        } else {
            // 如果需要更多颜色，循环使用
            let mut result = Vec::new();
            for i in 0..n {
                result.push(self.colors[i % self.colors.len()].clone());
            }
            result
        }
    }

    /// 获取指定索引的颜色
    pub fn get_color(&self, index: usize) -> &str {
        &self.colors[index % self.colors.len()]
    }
}

// McKinsey 配色方案
lazy_static! {
    pub static ref COLOR_SCHEMES: HashMap<&'static str, ColorScheme> = {
        let mut map = HashMap::new();
        
        // McKinsey 经典配色
        map.insert("mckinsey", ColorScheme {
            name: "McKinsey".to_string(),
            primary: "#1A1A1A".to_string(),
            secondary: "#4A4A4A".to_string(),
            accent: "#E87D0D".to_string(),
            background: "#F5F5F5".to_string(),
            text: "#333333".to_string(),
            colors: vec![
                "#1A1A1A".to_string(),
                "#4A4A4A".to_string(),
                "#7A7A7A".to_string(),
                "#A8A8A8".to_string(),
                "#D0D0D0".to_string(),
                "#E87D0D".to_string(),
                "#FFA726".to_string(),
                "#FFCC80".to_string(),
            ],
        });

        // BCG 配色
        map.insert("bcg", ColorScheme {
            name: "BCG".to_string(),
            primary: "#003366".to_string(),
            secondary: "#0066CC".to_string(),
            accent: "#FF6B35".to_string(),
            background: "#F8FAFC".to_string(),
            text: "#1E293B".to_string(),
            colors: vec![
                "#003366".to_string(),
                "#0066CC".to_string(),
                "#3399FF".to_string(),
                "#66B2FF".to_string(),
                "#99CCFF".to_string(),
                "#FF6B35".to_string(),
                "#FF8C42".to_string(),
                "#FFAD60".to_string(),
            ],
        });

        // Bain 配色
        map.insert("bain", ColorScheme {
            name: "Bain".to_string(),
            primary: "#002D56".to_string(),
            secondary: "#004D80".to_string(),
            accent: "#FFC107".to_string(),
            background: "#FFFFFF".to_string(),
            text: "#2C3E50".to_string(),
            colors: vec![
                "#002D56".to_string(),
                "#004D80".to_string(),
                "#006BA3".to_string(),
                "#008FC6".to_string(),
                "#4DB6AC".to_string(),
                "#FFC107".to_string(),
                "#FFD54F".to_string(),
                "#FFECB3".to_string(),
            ],
        });

        // EY 配色
        map.insert("ey", ColorScheme {
            name: "EY".to_string(),
            primary: "#1A73E8".to_string(),
            secondary: "#5F6368".to_string(),
            accent: "#FF5252".to_string(),
            background: "#F5F5F5".to_string(),
            text: "#202124".to_string(),
            colors: vec![
                "#1A73E8".to_string(),
                "#5F6368".to_string(),
                "#34A853".to_string(),
                "#FBBC05".to_string(),
                "#EA4335".to_string(),
                "#9C27B0".to_string(),
                "#FF5722".to_string(),
                "#00BCD4".to_string(),
            ],
        });

        // 腾讯配色
        map.insert("tencent", ColorScheme {
            name: "Tencent".to_string(),
            primary: "#1DA462".to_string(),
            secondary: "#0D7E50".to_string(),
            accent: "#FF6A00".to_string(),
            background: "#F5F5F5".to_string(),
            text: "#333333".to_string(),
            colors: vec![
                "#1DA462".to_string(),
                "#0D7E50".to_string(),
                "#0B5F3C".to_string(),
                "#2ED07A".to_string(),
                "#68E3A1".to_string(),
                "#FF6A00".to_string(),
                "#FF8C42".to_string(),
                "#FFB366".to_string(),
            ],
        });

        // 阿里配色
        map.insert("alibaba", ColorScheme {
            name: "Alibaba".to_string(),
            primary: "#FF4400".to_string(),
            secondary: "#CC3300".to_string(),
            accent: "#FFAA00".to_string(),
            background: "#FFFFFF".to_string(),
            text: "#333333".to_string(),
            colors: vec![
                "#FF4400".to_string(),
                "#CC3300".to_string(),
                "#992200".to_string(),
                "#FF6633".to_string(),
                "#FF8866".to_string(),
                "#FFAA00".to_string(),
                "#FFCC66".to_string(),
                "#FFE599".to_string(),
            ],
        });

        map
    };
}

/// 获取配色方案
pub fn get_color_scheme(name: &str) -> &'static ColorScheme {
    COLOR_SCHEMES.get(name).unwrap_or_else(|| {
        COLOR_SCHEMES.get("mckinsey").unwrap()
    })
}