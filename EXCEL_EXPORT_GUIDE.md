# Excel 导出功能实现总结

## 概述

成功实现了 tauri-vue-bi 项目的 **Excel 导出功能**，使用 `rust_xlsxwriter` 库将 Polars DataFrame 数据导出为标准 Excel (.xlsx) 文件。

## 技术实现

### 核心依赖

- **rust_xlsxwriter**: 纯 Rust 实现的 Excel 写入库
- **polars**: DataFrame 数据处理
- **chrono**: 日期时间处理

### 功能特性

✅ **完整的数据类型支持**:
- 字符串 (String)
- 整数 (Int8/16/32/64, UInt8/16/32/64)
- 浮点数 (Float32/64)
- 布尔值 (Boolean)
- 日期 (Date)
- 日期时间 (Datetime)

✅ **格式化功能**:
- 标题行加粗和灰色背景
- 边框样式
- 自动列宽调整
- 日期时间格式（yyyy-mm-dd, yyyy-mm-dd hh:mm:ss）

✅ **智能处理**:
- 自动生成文件名（带时间戳）
- 自定义文件名支持
- 空值安全处理
- 未知类型自动转换为字符串

## API 使用

### 通过 LLM 工具调用

```json
{
  "name": "export_excel",
  "arguments": {
    "tables": ["sales_data"],
    "filename": "销售数据_2026Q1.xlsx"
  }
}
```

### 直接调用函数

```rust
use crate::agent::tools::export_tools;

let result = export_tools::tool_export_excel(
    vec!["sales_data".to_string()],
    Some("my_export.xlsx".to_string())
)?;

println!("文件路径: {}", result.file_path);
println!("消息: {}", result.message);
```

## 返回结果

```rust
pub struct ExcelExportResult {
    pub file_path: String,     // 文件路径
    pub tables: Vec<String>,   // 包含的表格列表
    pub message: String,       // 导出消息
}
```

示例输出：
```
Excel 文件已导出: export_20260516_183045.xlsx (1000 行, 5 列)
```

## 实现细节

### 1. 数据类型映射

| Polars 类型 | Excel 类型 | 处理方式 |
|------------|-----------|---------|
| String | 文本 | 直接写入 |
| Int8/16/32/64 | 数字 | 转换为 f64 |
| UInt8/16/32/64 | 数字 | 转换为 f64 |
| Float32/64 | 数字 | 直接写入 |
| Boolean | 布尔值 | 直接写入 |
| Date | 日期 | 转换为 Excel 日期格式 |
| Datetime | 日期时间 | 转换为 Excel 日期时间格式 |
| 其他 | 文本 | 转换为字符串 |

### 2. 日期时间转换

**Date 类型**:
```rust
// Polars Date 是从 epoch (1970-01-01) 开始的天数
let date = chrono::NaiveDate::from_num_days_from_ce_opt(value + 719468)?;
worksheet.write_datetime_with_format(
    row, col,
    &rust_xlsxwriter::DateTime::from_naive_date(&date),
    &Format::new().set_num_format("yyyy-mm-dd")
)?;
```

**Datetime 类型**:
```rust
// Polars Datetime 是微秒时间戳
let datetime = chrono::NaiveDateTime::from_timestamp_millis(value / 1000)?;
worksheet.write_datetime_with_format(
    row, col,
    &rust_xlsxwriter::DateTime::from_naive_datetime(&datetime),
    &Format::new().set_num_format("yyyy-mm-dd hh:mm:ss")
)?;
```

### 3. 格式化设置

**标题行格式**:
```rust
let header_format = Format::new()
    .set_bold()                    // 加粗
    .set_background_color(Color::Gray)  // 灰色背景
    .set_border(FormatBorder::Thin);    // 细边框
```

**自动列宽**:
```rust
for (col_idx, col_name) in columns.iter().enumerate() {
    let width = col_name.len().max(15) as u16; // 最小宽度 15
    worksheet.set_column_width(col_idx as u16, width)?;
}
```

## 示例

### 示例 1: 基本导出

```rust
let result = tool_export_excel(vec!["data".to_string()], None)?;
// 输出: export_20260516_183045.xlsx
```

### 示例 2: 自定义文件名

```rust
let result = tool_export_excel(
    vec!["sales".to_string()],
    Some("Q1_Sales_Report.xlsx".to_string())
)?;
// 输出: Q1_Sales_Report.xlsx
```

### 示例 3: 包含多种数据类型

假设 DataFrame 包含：
```
| name   | age | salary | hire_date  | is_active |
|--------|-----|--------|------------|-----------|
| Alice  | 30  | 5000.5 | 2020-01-15 | true      |
| Bob    | 25  | 4500.0 | 2021-03-20 | false     |
```

导出的 Excel 文件将正确保留所有数据类型和格式。

## 性能特点

### 优势

1. **纯 Rust 实现**: 无需 Python 或其他运行时
2. **内存效率**: 流式写入，适合大数据集
3. **类型安全**: 编译时捕获类型错误
4. **跨平台**: Windows/macOS/Linux 统一行为

### 限制

1. **单工作表**: 当前只支持单个工作表
2. **无公式**: 不支持 Excel 公式
3. **无图表**: 不支持嵌入图表（需要单独生成）
4. **无宏**: 不支持 VBA 宏

## 未来扩展

计划添加的功能：

- [ ] 多工作表支持（每个 table 一个工作表）
- [ ] Excel 公式支持
- [ ] 条件格式化
- [ ] 数据验证
- [ ] 图表嵌入
- [ ] 透视表
- [ ] 冻结窗格
- [ ] 打印设置

## 与 Python 版本对比

| 特性 | Python (openpyxl/pandas) | Rust (rust_xlsxwriter) |
|------|-------------------------|------------------------|
| 性能 | 中等（需要 Python 运行时） | 快（原生代码） |
| 内存 | 较高 | 较低 |
| 类型安全 | 动态类型 | 静态类型 |
| 部署 | 需要 Python 环境 | 单一二进制文件 |
| 功能完整性 | 完整 | 基础功能（持续完善中） |

## 注意事项

1. **文件路径**: 导出文件保存在当前工作目录
2. **覆盖警告**: 如果文件已存在会被覆盖
3. **大数据集**: 对于超大数据集（>100万行），建议分批导出
4. **日期范围**: Excel 支持的日期范围是 1900-01-01 到 9999-12-31

## 测试建议

### 单元测试

```rust
#[test]
fn test_export_excel_basic() {
    // 创建测试 DataFrame
    let df = df! {
        "name" => &["Alice", "Bob"],
        "age" => &[30, 25],
        "salary" => &[5000.5, 4500.0],
    }.unwrap();
    
    // 设置全局 DataFrame
    // ...
    
    // 导出 Excel
    let result = tool_export_excel(vec!["test".to_string()], Some("test.xlsx".to_string())).unwrap();
    
    // 验证文件存在
    assert!(PathBuf::from(&result.file_path).exists());
    
    // 清理
    std::fs::remove_file(&result.file_path).ok();
}
```

### 集成测试

```rust
#[tokio::test]
async fn test_agent_excel_export() {
    // 创建会话
    let session_id = create_session("openai").await?;
    
    // 发送导出命令
    let events = chat_stream(
        session_id,
        "/export 导出数据".to_string(),
        // ...
    ).await?;
    
    // 验证收到 ExcelOutline 事件
    // ...
}
```

---

**作者**: alex  
**日期**: 2026-05-16  
**版本**: 1.0
