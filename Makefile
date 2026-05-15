SHELL := /bin/bash

PY_AGENT_DIR := Data-Analysis-Agent
PY_VENV_DIR := $(PY_AGENT_DIR)/.venv
PYTHON_BIN ?= $(abspath $(PY_VENV_DIR)/bin/python)

# ── 颜色输出 ──────────────────────────────────────────────────────────────────
BOLD  := \033[1m
RESET := \033[0m
GREEN := \033[32m
CYAN  := \033[36m
YELLOW:= \033[33m

.PHONY: help install dev build bundle dmg test test-rust test-ts lint fmt clean \
	check-deps python-setup python-verify icon update-deps release-check release-tag release-tag-fix release-push release

# ─────────────────────────────────────────────────────────────────────────────
# 默认目标：帮助信息
# ─────────────────────────────────────────────────────────────────────────────
help:
	@printf "$(BOLD)tauri-vue-bi Makefile$(RESET)\n\n"
	@printf "$(CYAN)开发$(RESET)\n"
	@printf "  make install      安装前端依赖 (npm ci)\n"
	@printf "  make python-setup 使用 uv 初始化 Python 环境（Data-Analysis-Agent/.venv）\n"
	@printf "  make python-verify 验证 Python 解释器和关键依赖\n"
	@printf "  make dev          启动开发模式（Tauri + Vite 热重载）\n"
	@printf "\n$(CYAN)构建 & 打包$(RESET)\n"
	@printf "  make build        构建前端（vite build + vue-tsc 类型检查）\n"
	@printf "  make icon         重新生成 Tauri 图标资源（ico/icns/png）\n"
	@printf "  make bundle       完整打包桌面应用（生成 .dmg/.app/.deb/.exe 等）\n"
	@printf "  make dmg          仅构建 macOS DMG 安装包\n"
	@printf "  make release-check 发布前检查（build + test-rust）\n"
	@printf "  make release-tag TAG=tauri-vue-bi-v0.1.0   创建发布 tag\n"
	@printf "                    （若 tag 已存在，会给出可选修复命令）\n"
	@printf "  make release-tag-fix TAG=tauri-vue-bi-v0.1.0 FIX=3\n"
	@printf "                    自动执行冲突修复（FIX=1/2/3）\n"
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
	@command -v uv      >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) uv      $$(uv --version | head -n 1)\n" || printf "  $(YELLOW)!$(RESET) uv 未安装（推荐用于 Python 环境管理）\n"
	@command -v tauri   >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) tauri   $$(tauri --version)\n"  || \
	  (npx tauri --version >/dev/null 2>&1 && printf "  $(GREEN)✔$(RESET) tauri   (via npx)\n"         || printf "  $(YELLOW)!$(RESET) tauri CLI 未找到（可通过 npm run tauri 调用）\n")

# ─────────────────────────────────────────────────────────────────────────────
# Python 环境（uv + .venv）
# ─────────────────────────────────────────────────────────────────────────────
python-setup:
	@printf "$(BOLD)初始化 Python 环境（uv）...$(RESET)\n"
	@command -v uv >/dev/null 2>&1 || (printf "$(YELLOW)!$(RESET) 未找到 uv，请先安装 uv 后再执行\n" && exit 1)
	@test -f $(PY_AGENT_DIR)/pyproject.toml || (printf "$(YELLOW)!$(RESET) 缺少 $(PY_AGENT_DIR)/pyproject.toml\n" && exit 1)
	uv sync --project $(PY_AGENT_DIR)
	@test -x "$(PYTHON_BIN)" || (printf "$(YELLOW)!$(RESET) 初始化后仍未找到解释器: $(PYTHON_BIN)\n" && exit 1)
	@printf "  $(GREEN)✔$(RESET) Python 环境已就绪: $(PYTHON_BIN)\n"

python-verify:
	@printf "$(BOLD)验证 Python 环境...$(RESET)\n"
	@if [ ! -x "$(PYTHON_BIN)" ]; then \
	  printf "  $(YELLOW)!$(RESET) 未找到解释器 $(PYTHON_BIN)，尝试自动初始化...\n"; \
	  $(MAKE) --no-print-directory python-setup; \
	fi
	@test -x "$(PYTHON_BIN)" || (printf "$(YELLOW)!$(RESET) 无法使用 Python 解释器: $(PYTHON_BIN)\n" && exit 1)
	@"$(PYTHON_BIN)" --version
	@"$(PYTHON_BIN)" -c "import flask, flask_cors, pandas, openpyxl, sqlalchemy, requests; print('Python deps OK: flask/flask_cors/pandas/openpyxl/sqlalchemy/requests')"

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
dev: python-verify
	@printf "$(BOLD)启动 Tauri 开发模式（Vite 热重载 + Rust 增量编译）...$(RESET)\n"
	PYTHON_BIN="$(PYTHON_BIN)" npm run tauri -- dev

# ─────────────────────────────────────────────────────────────────────────────
# 构建
# ─────────────────────────────────────────────────────────────────────────────

# 仅构建前端（TypeScript 检查 + vite build）
build:
	@printf "$(BOLD)构建前端...$(RESET)\n"
	npm run build

# 重新生成 Tauri 图标（避免损坏/空文件导致打包失败）
icon:
	@test -f src-tauri/icons/icon-source.svg || (printf "$(YELLOW)!$(RESET) 缺少 src-tauri/icons/icon-source.svg\n" && exit 1)
	@printf "$(BOLD)生成 Tauri 图标资源...$(RESET)\n"
	npm run tauri -- icon src-tauri/icons/icon-source.svg

# 完整打包：前端 build + Rust release 编译 + 平台安装包
bundle: icon
	@printf "$(BOLD)打包桌面应用（release 模式）...$(RESET)\n"
	npm run tauri -- build

# 仅构建 macOS DMG 安装包
dmg: icon
	@printf "$(BOLD)构建 macOS DMG 安装包（release 模式）...$(RESET)\n"
	@uname | grep -q "Darwin" || (printf "$(YELLOW)!$(RESET) 当前非 macOS，无法构建 DMG\n" && exit 1)
	npm run tauri -- build --bundles dmg

release-check: build test-rust
	@printf "$(GREEN)✔$(RESET) 发布前检查通过\n"

release-tag:
	@test -n "$(TAG)" || (printf "$(YELLOW)!$(RESET) 用法: make release-tag TAG=tauri-vue-bi-v0.1.0\n" && exit 1)
	@git rev-parse --is-inside-work-tree >/dev/null 2>&1 || (printf "$(YELLOW)!$(RESET) 当前目录不在 git 仓库中\n" && exit 1)
	@local_exists=0; remote_exists=0; \
	git rev-parse -q --verify "refs/tags/$(TAG)" >/dev/null 2>&1 && local_exists=1 || true; \
	git ls-remote --tags origin "refs/tags/$(TAG)" "refs/tags/$(TAG)^{}" | grep -q . && remote_exists=1 || true; \
	if [ $$local_exists -eq 1 ] || [ $$remote_exists -eq 1 ]; then \
	  printf "$(YELLOW)!$(RESET) 检测到发布 tag 冲突: $(TAG)\n"; \
	  [ $$local_exists -eq 1 ] && printf "  - 本地已存在同名 tag\n" || true; \
	  [ $$remote_exists -eq 1 ] && printf "  - 远端 origin 已存在同名 tag\n" || true; \
	  printf "\n可选修复命令（按需选择其一执行）：\n"; \
	  printf "  [1] 仅推送本地 tag（适用于本地有、远端无）\n"; \
	  printf "      git push origin \"$(TAG)\"\n\n"; \
	  printf "  [2] 在当前提交重建本地 tag（不动远端）\n"; \
	  printf "      git tag -d \"$(TAG)\"\n"; \
	  printf "      git tag -a \"$(TAG)\" -m \"Release $(TAG)\"\n\n"; \
	  printf "  [3] 强制重建并同步远端（高风险，确保团队已知）\n"; \
	  printf "      git tag -d \"$(TAG)\"\n"; \
	  printf "      git push origin :refs/tags/$(TAG)\n"; \
	  printf "      git tag -a \"$(TAG)\" -m \"Release $(TAG)\"\n"; \
	  printf "      git push origin \"$(TAG)\"\n"; \
	  exit 1; \
	fi
	@printf "$(BOLD)创建发布 tag: $(TAG)$(RESET)\n"
	git tag -a "$(TAG)" -m "Release $(TAG)"

release-tag-fix:
	@test -n "$(TAG)" || (printf "$(YELLOW)!$(RESET) 用法: make release-tag-fix TAG=tauri-vue-bi-v0.1.0 FIX=3\n" && exit 1)
	@test -n "$(FIX)" || (printf "$(YELLOW)!$(RESET) 用法: make release-tag-fix TAG=tauri-vue-bi-v0.1.0 FIX=1|2|3\n" && exit 1)
	@git rev-parse --is-inside-work-tree >/dev/null 2>&1 || (printf "$(YELLOW)!$(RESET) 当前目录不在 git 仓库中\n" && exit 1)
	@if [ "$(FIX)" = "1" ]; then \
	  printf "$(BOLD)执行修复 [1]：仅推送本地 tag$(RESET)\n"; \
	  git push origin "$(TAG)"; \
	elif [ "$(FIX)" = "2" ]; then \
	  printf "$(BOLD)执行修复 [2]：在当前提交重建本地 tag$(RESET)\n"; \
	  git tag -d "$(TAG)" >/dev/null 2>&1 || true; \
	  git tag -a "$(TAG)" -m "Release $(TAG)"; \
	elif [ "$(FIX)" = "3" ]; then \
	  printf "$(BOLD)执行修复 [3]：强制重建并同步远端$(RESET)\n"; \
	  git tag -d "$(TAG)" >/dev/null 2>&1 || true; \
	  git push origin :refs/tags/$(TAG) >/dev/null 2>&1 || true; \
	  git tag -a "$(TAG)" -m "Release $(TAG)"; \
	  git push origin "$(TAG)"; \
	else \
	  printf "$(YELLOW)!$(RESET) FIX 参数无效：$(FIX)（仅支持 1/2/3）\n"; \
	  exit 1; \
	fi

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
