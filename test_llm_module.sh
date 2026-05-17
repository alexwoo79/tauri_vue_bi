#!/bin/bash

# LLM 客户端测试脚本

echo "======================================"
echo "LLM 客户端模块测试"
echo "======================================"
echo ""

# 检查 OPENAI_API_KEY
if [ -z "$OPENAI_API_KEY" ]; then
    echo "⚠️  警告: 未设置 OPENAI_API_KEY 环境变量"
    echo "   如需测试 OpenAI，请运行: export OPENAI_API_KEY='sk-...'"
    echo ""
else
    echo "✅ OPENAI_API_KEY 已设置"
fi

# 检查 ANTHROPIC_API_KEY
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "⚠️  警告: 未设置 ANTHROPIC_API_KEY 环境变量"
    echo "   如需测试 Claude，请运行: export ANTHROPIC_API_KEY='sk-ant-...'"
    echo ""
else
    echo "✅ ANTHROPIC_API_KEY 已设置"
fi

echo ""
echo "======================================"
echo "运行单元测试"
echo "======================================"
echo ""

# 运行 LLM 模块的单元测试（跳过需要 API key 的测试）
cd src-tauri
cargo test --lib llm --no-fail-fast

echo ""
echo "======================================"
echo "编译检查"
echo "======================================"
echo ""

# 检查编译
cargo check

echo ""
echo "======================================"
echo "测试完成！"
echo "======================================"
