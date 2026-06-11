param(
  [switch]$SkipRust,
  [switch]$SkipFrontend
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot\..

function Require-Tool([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required tool: $Name"
  }
}

if (-not $SkipFrontend) {
  Require-Tool pnpm
}

if (-not $SkipRust) {
  Require-Tool cargo
}

$python = 'C:\Python310\python.exe'
if (-not (Test-Path $python)) {
  throw "Expected Python not found at $python"
}

if ($SkipRust) {
  Write-Host '[submission-check] skipping Rust checks (SkipRust enabled)'
} else {
  Write-Host '[submission-check] cargo check (excluding desktop Tauri shell)'
  cargo check --workspace --exclude synthesize-ide-desktop

  Write-Host '[submission-check] cargo test (excluding desktop Tauri shell)'
  cargo test --workspace --exclude synthesize-ide-desktop
}

if ($SkipFrontend) {
  Write-Host '[submission-check] skipping frontend checks (SkipFrontend enabled)'
} else {
  Write-Host '[submission-check] pnpm install --no-frozen-lockfile'
  pnpm install --no-frozen-lockfile

  Write-Host '[submission-check] pnpm build'
  pnpm build

  Write-Host '[submission-check] pnpm typecheck'
  pnpm typecheck

  Write-Host '[submission-check] pnpm test'
  pnpm test
}

Write-Host '[submission-check] MoA bridge self-test'
& $python integrations\moa\synthesize_bridge.py --self-test

Write-Host '[submission-check] MoA bridge pytest'
& $python -m pytest integrations\moa\tests\test_synthesize_bridge.py -q

Write-Host '[submission-check] PASS'