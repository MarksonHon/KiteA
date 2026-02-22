#!/usr/bin/env pwsh
<#
.SYNOPSIS
    KiteA 自动化构建脚本（Windows / PowerShell）

.PARAMETER Release
    执行 cargo build --release；省略时执行 cargo check

.PARAMETER FrontendOnly
    仅构建前端，跳过 Rust 编译

.PARAMETER BackendOnly
    仅执行 Rust 编译，跳过前端构建（frontend/dist 须已存在）

.PARAMETER SkipInstall
    跳过 npm install（已有 node_modules 时加速构建）

.EXAMPLE
    .\build.ps1                  # 全量检查模式（cargo check）
    .\build.ps1 -Release         # 全量发布构建
    .\build.ps1 -Release -SkipInstall
    .\build.ps1 -FrontendOnly
#>

param(
    [switch]$Release,
    [switch]$FrontendOnly,
    [switch]$BackendOnly,
    [switch]$SkipInstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── 颜色辅助 ──────────────────────────────────────────────────────────────────
function Write-Step  { param($msg) Write-Host "`n==> $msg" -ForegroundColor Cyan  }
function Write-Ok    { param($msg) Write-Host "    $msg"   -ForegroundColor Green }
function Write-Fail  { param($msg) Write-Host "    $msg"   -ForegroundColor Red; exit 1 }

# ── 路径 ──────────────────────────────────────────────────────────────────────
$Root        = $PSScriptRoot
$FrontendDir = Join-Path $Root "frontend"
$DistDir     = Join-Path $FrontendDir "dist"

# ── MinGW（仅 Windows，x86_64-pc-windows-gnu 工具链需要 dlltool.exe） ────────
$MingwCandidates = @(
    "$env:USERPROFILE\scoop\apps\mingw-winlibs-llvm-ucrt\current\bin",
    "$env:USERPROFILE\scoop\apps\mingw\current\bin",
    "C:\mingw64\bin",
    "C:\msys64\mingw64\bin"
)
foreach ($candidate in $MingwCandidates) {
    if (Test-Path (Join-Path $candidate "dlltool.exe")) {
        if ($env:PATH -notlike "*$candidate*") {
            $env:PATH = "$candidate;$env:PATH"
            Write-Ok "MinGW 已添加到 PATH: $candidate"
        } else {
            Write-Ok "MinGW 已在 PATH 中: $candidate"
        }
        break
    }
}

# 检查工具链
foreach ($tool in @("cargo", "npm")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Write-Fail "$tool 未找到，请确认已正确安装。"
    }
}

# ── 前端构建 ──────────────────────────────────────────────────────────────────
if (-not $BackendOnly) {
    Write-Step "前端构建（Vue 3 + Vite）"
    Push-Location $FrontendDir
    try {
        if (-not $SkipInstall) {
            Write-Host "  >> npm install"
            npm install
            if ($LASTEXITCODE -ne 0) { Write-Fail "npm install 失败 (exit $LASTEXITCODE)" }
        }

        Write-Host "  >> npm run build"
        npm run build
        if ($LASTEXITCODE -ne 0) { Write-Fail "npm run build 失败 (exit $LASTEXITCODE)" }

        $assetCount = (Get-ChildItem $DistDir -Recurse -File).Count
        Write-Ok "前端构建成功，共 $assetCount 个文件 → $DistDir"
    } finally {
        Pop-Location
    }
}

# ── Rust 构建 ─────────────────────────────────────────────────────────────────
if (-not $FrontendOnly) {
    if (-not (Test-Path $DistDir)) {
        Write-Fail "frontend/dist 不存在，rust-embed 需要先构建前端。运行时加 -FrontendOnly 或去掉 -BackendOnly。"
    }

    Push-Location $Root
    try {
        if ($Release) {
            Write-Step "Rust 发布构建（cargo build --release）"
            cargo build --release
            if ($LASTEXITCODE -ne 0) { Write-Fail "cargo build --release 失败 (exit $LASTEXITCODE)" }

            $binary = Join-Path $Root "target\release\kite_a.exe"
            $size   = [math]::Round((Get-Item $binary).Length / 1MB, 1)
            Write-Ok "构建成功 → $binary  ($size MB)"
        } else {
            Write-Step "Rust 检查（cargo check）"
            cargo check
            if ($LASTEXITCODE -ne 0) { Write-Fail "cargo check 失败 (exit $LASTEXITCODE)" }
            Write-Ok "cargo check 通过"
        }
    } finally {
        Pop-Location
    }
}

Write-Host ""
Write-Ok "========== 构建完成 =========="
if ($Release -and -not $FrontendOnly) {
    Write-Host "    运行：.\target\release\kite_a.exe" -ForegroundColor White
    Write-Host "    然后访问：http://localhost:2026" -ForegroundColor White
}
