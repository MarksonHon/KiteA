#!/usr/bin/env bash
# KiteA 自动化构建脚本（Linux / macOS / WSL）
#
# 用法：
#   ./build.sh                  全量检查模式（cargo check）
#   ./build.sh --release        全量发布构建
#   ./build.sh --frontend-only  仅构建前端
#   ./build.sh --backend-only   仅执行 Rust 编译（frontend/dist 须已存在）
#   ./build.sh --release --skip-install  跳过 npm install

set -euo pipefail

# ── 参数解析 ──────────────────────────────────────────────────────────────────
RELEASE=false
FRONTEND_ONLY=false
BACKEND_ONLY=false
SKIP_INSTALL=false

for arg in "$@"; do
    case "$arg" in
        --release)        RELEASE=true ;;
        --frontend-only)  FRONTEND_ONLY=true ;;
        --backend-only)   BACKEND_ONLY=true ;;
        --skip-install)   SKIP_INSTALL=true ;;
        -h|--help)
            sed -n '2,12p' "$0" | sed 's/^# //'
            exit 0 ;;
        *)
            echo "未知参数: $arg" >&2; exit 1 ;;
    esac
done

# ── 颜色 ──────────────────────────────────────────────────────────────────────
CYAN='\033[0;36m'; GREEN='\033[0;32m'; RED='\033[0;31m'; RESET='\033[0m'
step() { echo -e "\n${CYAN}==> $*${RESET}"; }
ok()   { echo -e "    ${GREEN}$*${RESET}"; }
fail() { echo -e "    ${RED}$*${RESET}" >&2; exit 1; }

# ── 路径 ──────────────────────────────────────────────────────────────────────
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$ROOT/frontend"
DIST_DIR="$FRONTEND_DIR/dist"

# ── 工具检查 ──────────────────────────────────────────────────────────────────
for tool in cargo npm node; do
    command -v "$tool" &>/dev/null || fail "$tool 未找到，请先安装。"
done

# ── 前端构建 ──────────────────────────────────────────────────────────────────
if [[ "$BACKEND_ONLY" == false ]]; then
    step "前端构建（Vue 3 + Vite）"
    cd "$FRONTEND_DIR"

    if [[ "$SKIP_INSTALL" == false ]]; then
        echo "  >> npm install"
        npm install
    fi

    echo "  >> npm run build"
    npm run build

    asset_count=$(find "$DIST_DIR" -type f | wc -l | tr -d ' ')
    ok "前端构建成功，共 ${asset_count} 个文件 → $DIST_DIR"
    cd "$ROOT"
fi

# ── Rust 构建 ─────────────────────────────────────────────────────────────────
if [[ "$FRONTEND_ONLY" == false ]]; then
    [[ -d "$DIST_DIR" ]] || \
        fail "frontend/dist 不存在，rust-embed 编译需要先构建前端。\n请先运行: ./build.sh --frontend-only"

    cd "$ROOT"

    if [[ "$RELEASE" == true ]]; then
        step "Rust 发布构建（cargo build --release）"
        cargo build --release

        binary="$ROOT/target/release/kite_a"
        size=$(du -sh "$binary" | cut -f1)
        ok "构建成功 → $binary  ($size)"
    else
        step "Rust 检查（cargo check）"
        cargo check
        ok "cargo check 通过"
    fi
fi

echo ""
ok "========== 构建完成 =========="
if [[ "$RELEASE" == true && "$FRONTEND_ONLY" == false ]]; then
    echo -e "    运行：./target/release/kite_a"
    echo -e "    然后访问：http://localhost:2026"
fi
