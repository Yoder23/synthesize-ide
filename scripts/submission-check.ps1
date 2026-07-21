param(
  [switch]$SkipRust,
  [switch]$SkipFrontend,
  [string]$Python = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$env:CI = 'true'

Set-Location $PSScriptRoot\..

function Require-Tool([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required tool: $Name"
  }
}

function Assert-NativeSuccess([string]$Step) {
  if ($LASTEXITCODE -ne 0) {
    throw "$Step failed with exit code $LASTEXITCODE"
  }
}

function Resolve-PythonExecutable([string]$Preferred) {
  if ($Preferred -and (Test-Path -LiteralPath $Preferred)) {
    return (Resolve-Path -LiteralPath $Preferred).Path
  }
  $knownLocal = 'C:\Python310\python.exe'
  if (Test-Path -LiteralPath $knownLocal) {
    return $knownLocal
  }
  $launcher = Get-Command py -ErrorAction SilentlyContinue
  if ($launcher) {
    return 'py -3'
  }
  $python = Get-Command python -ErrorAction SilentlyContinue
  if ($python) {
    return 'python'
  }
  throw 'Python 3 was not found. Install Python 3 or pass -Python <path-to-python.exe>.'
}

if (-not $SkipFrontend) {
  Require-Tool pnpm
}

if (-not $SkipRust) {
  Require-Tool cargo
}

$pythonCmd = Resolve-PythonExecutable $Python

if ($SkipRust) {
  Write-Host '[submission-check] skipping Rust checks (SkipRust enabled)'
} else {
  Write-Host '[submission-check] cargo check (excluding desktop Tauri shell)'
  cargo check --workspace --exclude synthesize-ide-desktop
  Assert-NativeSuccess 'cargo check'

  Write-Host '[submission-check] cargo test (excluding desktop Tauri shell)'
  cargo test --workspace --exclude synthesize-ide-desktop
  Assert-NativeSuccess 'cargo test'

  Write-Host '[submission-check] cargo fmt --check'
  cargo fmt --all -- --check
  Assert-NativeSuccess 'cargo fmt --check'
}

if ($SkipFrontend) {
  Write-Host '[submission-check] skipping frontend checks (SkipFrontend enabled)'
} else {
  Write-Host '[submission-check] pnpm install --frozen-lockfile'
  pnpm install --frozen-lockfile
  Assert-NativeSuccess 'pnpm install'

  Write-Host '[submission-check] pnpm build'
  pnpm build
  Assert-NativeSuccess 'pnpm build'

  Write-Host '[submission-check] pnpm typecheck'
  pnpm typecheck
  Assert-NativeSuccess 'pnpm typecheck'

  Write-Host '[submission-check] pnpm test'
  pnpm test
  Assert-NativeSuccess 'pnpm test'
}

Write-Host '[submission-check] MoA bridge self-test'
if ($pythonCmd -eq 'py -3') {
  & py -3 integrations\moa\synthesize_bridge.py --self-test
} else {
  & $pythonCmd integrations\moa\synthesize_bridge.py --self-test
}
Assert-NativeSuccess 'MoA bridge self-test'

Write-Host '[submission-check] MoA verifier'
if ($pythonCmd -eq 'py -3') {
  & py -3 integrations\moa\verify_moa.py
} else {
  & $pythonCmd integrations\moa\verify_moa.py
}
Assert-NativeSuccess 'MoA verifier'

Write-Host '[submission-check] MoA pytest'
if ($pythonCmd -eq 'py -3') {
  & py -3 -m pytest integrations\moa\tests -q
} else {
  & $pythonCmd -m pytest integrations\moa\tests -q
}
Assert-NativeSuccess 'MoA pytest'

Write-Host '[submission-check] PASS'
