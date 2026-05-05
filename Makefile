SHELL := /bin/bash

# ── 颜色输出 ──────────────────────────────────────────────────────────────────
BOLD  := \033[1m
RESET := \033[0m
GREEN := \033[32m
CYAN  := \033[36m
YELLOW:= \033[33m

.PHONY: help install dev build bundle dmg test test-rust test-ts lint fmt clean \
	check-deps icon update-deps release-check release-tag release-push release

# ─────────────────────────────────────────────────────────────────────────────
# 默认目标：帮助信息
# ─────────────────────────────────────────────────────────────────────────────
help:
	@printf "$(BOLD)tauri-vue-bi Makefile$(RESET)\n\n"
	@printf "$(CYAN)开发$(RESET)\n"
	@printf "  make install      安装前端依赖 (npm ci)\n"
	@printf "  make dev          启动开发模式（Tauri + Vite 热重载）\n"
	@printf "\n$(CYAN)构建 & 打包$(RESET)\n"
	@printf "  make build        构建前端（vite build + vue-tsc 类型检查）\n"
	@printf "  make bundle       完整打包桌面应用（生成 .dmg/.app/.deb/.exe 等）\n"
	@printf "  make dmg          仅构建 macOS DMG 安装包\n"
	@printf "  make release-check 发布前检查（build + test-rust）\n"
	@printf "  make release-tag TAG=tauri-vue-bi-v0.1.0   创建发布 tag\n"
	@printf "  make release-push TAG=tauri-vue-bi-v0.1.0  推送发布 tag 到 GitHub\n"
	@printf "  make release TAG=tauri-vue-bi-v0.1.0       检查 + 打 tag + 推送\n"
	@printf "\n$(CYAN)测试$(RESET)\n"
	@printf "  make test         运行全部测试（Rust + TypeScript）\n"
	@printf "  make test-rust    仅运行 Rust 单元测试\n"
	@printf "  make test-ts      仅运行前端单元测试（vitest，如已配置）\n"
	@printf "\n$(CYAN)代码质量$(RESET)\n"
	@printf "  make lint         前端 ESLint + Rust clippy\n"
	@printf "  make fmt          格式化代码（rustfmt + prettier，如已安装）\n"
	@printf "  make check-deps   检查必要工具是否已安装\n"
	@printf "\n$(CYAN)维护$(RESET)\n"
	@printf "  make update-deps  更新前端依赖（npm update）\n"
	@printf "  make clean        清理构建产物\n"

# ─────────────────────────────────────────────────────────────────────────────
# 环境检查
# ─────────────────────────────────────────────────────────────────────────────
check-deps:
	@printf "$(BOLD)检查依赖工具...$(RESET)\n"
	@command -v node    >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) node    $$(node --version)\n"    || printf "  ✘ node 未安装\n"
	@command -v npm     >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) npm     $$(npm --version)\n"     || printf "  ✘ npm 未安装\n"
	@command -v rustc   >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) rustc   $$(rustc --version)\n"  || printf "  ✘ rustc 未安装\n"
	@command -v cargo   >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) cargo   $$(cargo --version)\n"  || printf "  ✘ cargo 未安装\n"
	@command -v tauri   >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) tauri   $$(tauri --version)\n"  || \
	  (npx tauri --version >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) tauri   (via npx)\n"         || printf "  $(YELLOW)!$(RESET) tauri CLI 未找到（可通过 npm run tauri 调用）\n")

# ─────────────────────────────────────────────────────────────────────────────
# 依赖安装
# ─────────────────────────────────────────────────────────────────────────────
install:
	@printf "$(BOLD)安装前端依赖...$(RESET)\n"
	npm ci

update-deps:
	@printf "$(BOLD)更新前端依赖...$(RESET)\n"
	npm update

# ─────────────────────────────────────────────────────────────────────────────
# 开发模式
# ─────────────────────────────────────────────────────────────────────────────
dev:
	@printf "$(BOLD)启动 Tauri 开发模式（Vite 热重载 + Rust 增量编译）...$(RESET)\n"
	npm run tauri -- dev

# ─────────────────────────────────────────────────────────────────────────────
# 构建
# ─────────────────────────────────────────────────────────────────────────────

# 仅构建前端（TypeScript 检查 + vite build）
build:
	@printf "$(BOLD)构建前端...$(RESET)\n"
	npm run build

# 完整打包：前端 build + Rust release 编译 + 平台安装包
bundle:
	@printf "$(BOLD)打包桌面应用（release 模式）...$(RESET)\n"
	npm run tauri -- build

# 仅构建 macOS DMG 安装包
dmg:
	@printf "$(BOLD)构建 macOS DMG 安装包（release 模式）...$(RESET)\n"
	@uname | grep -q "Darwin" || (printf "$(YELLOW)!$(RESET) 当前非 macOS，无法构建 DMG\n" && exit 1)
	npm run tauri -- build --bundles dmg

release-check: build test-rust
	@printf "$(GREEN)✔$(RESET) 发布前检查通过\n"

release-tag:
	@test -n "$(TAG)" || (printf "$(YELLOW)!$(RESET) 用法: make release-tag TAG=tauri-vue-bi-v0.1.0\n" && exit 1)
	@git rev-parse --is-inside-work-tree >/dev/null 2>&1 || (printf "$(YELLOW)!$(RESET) 当前目录不在 git 仓库中\n" && exit 1)
	@git rev-parse "$(TAG)" >/dev/null 2>&1 && (printf "$(YELLOW)!$(RESET) tag $(TAG) 已存在\n" && exit 1) || true
	@printf "$(BOLD)创建发布 tag: $(TAG)$(RESET)\n"
	git tag -a "$(TAG)" -m "Release $(TAG)"

release-push:
	@test -n "$(TAG)" || (printf "$(YELLOW)!$(RESET) 用法: make release-push TAG=tauri-vue-bi-v0.1.0\n" && exit 1)
	@printf "$(BOLD)推送 tag 到 origin: $(TAG)$(RESET)\n"
	git push origin "$(TAG)"

release: release-check release-tag release-push
	@printf "$(GREEN)✔$(RESET) Release tag 已推送，GitHub Actions 将开始构建并发布多平台安装包\n"

# ─────────────────────────────────────────────────────────────────────────────
# 测试
# ─────────────────────────────────────────────────────────────────────────────
test: test-rust test-ts

test-rust:
	@printf "$(BOLD)运行 Rust 单元测试...$(RESET)\n"
	cd src-tauri && cargo test

test-ts:
	@printf "$(BOLD)运行前端测试（vitest）...$(RESET)\n"
	@if npm run --silent 2>/dev/null | grep -q "^  test$$"; then \
	    npm run test; \
	else \
	    printf "$(YELLOW)  跳过：package.json 中未定义 test 脚本$(RESET)\n"; \
	fi

# ─────────────────────────────────────────────────────────────────────────────
# 代码质量
# ─────────────────────────────────────────────────────────────────────────────
lint:
	@printf "$(BOLD)前端 ESLint...$(RESET)\n"
	@if npm run --silent 2>/dev/null | grep -q "^  lint$$"; then \
	    npm run lint; \
	else \
	    printf "$(YELLOW)  跳过：package.json 中未定义 lint 脚本$(RESET)\n"; \
	fi
	@printf "$(BOLD)Rust clippy...$(RESET)\n"
	cd src-tauri && cargo clippy -- -D warnings

fmt:
	@printf "$(BOLD)格式化 Rust 代码 (rustfmt)...$(RESET)\n"
	cd src-tauri && cargo fmt
	@printf "$(BOLD)格式化前端代码 (prettier)...$(RESET)\n"
	@if command -v prettier >/dev/null 2>&1; then \
	    prettier --write "src/**/*.{ts,vue}"; \
	else \
	    printf "$(YELLOW)  跳过：prettier 未安装$(RESET)\n"; \
	fi

# ─────────────────────────────────────────────────────────────────────────────
# 清理
# ─────────────────────────────────────────────────────────────────────────────
clean:
	@printf "$(BOLD)清理构建产物...$(RESET)\n"
	rm -rf dist node_modules
	cd src-tauri && cargo clean
	@printf "  $(GREEN)✔$(RESET) 清理完成\n"
