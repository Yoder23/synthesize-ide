param(
  [int]$Port = 8080,
  [switch]$SkipRust,
  [switch]$SkipFrontend,
  [string]$Python = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot\..

function Resolve-PythonExecutable([string]$Preferred) {
  if ($Preferred -and (Test-Path -LiteralPath $Preferred)) {
    return (Resolve-Path -LiteralPath $Preferred).Path
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

function Run-PythonCommand([string]$PythonCmd, [string[]]$Args) {
  if ($PythonCmd -eq 'py -3') {
    & py -3 @Args
  } else {
    & $PythonCmd @Args
  }
}

function Check-Tool([string]$Name, [switch]$Required) {
  $cmd = Get-Command $Name -ErrorAction SilentlyContinue
  if ($cmd) {
    Write-Host "[preflight] OK: found $Name at $($cmd.Source)"
    return $true
  }
  if ($Required) {
    throw "[preflight] MISSING required tool: $Name"
  }
  Write-Warning "[preflight] missing optional tool: $Name"
  return $false
}

function Test-PortInUse([int]$PortToCheck) {
  $conn = Get-NetTCPConnection -LocalPort $PortToCheck -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
  return $null -ne $conn
}

Write-Host '[preflight] checking local demo prerequisites'

if (-not $SkipFrontend) {
  Check-Tool -Name 'pnpm' -Required
} else {
  Write-Host '[preflight] frontend tool checks skipped'
}

if (-not $SkipRust) {
  Check-Tool -Name 'cargo' -Required
} else {
  Write-Host '[preflight] rust tool checks skipped'
}

$pythonCmd = Resolve-PythonExecutable $Python
Write-Host "[preflight] OK: python command resolved to '$pythonCmd'"

$configPath = '.synthesize-runtime\local-model.json'
if (-not (Test-Path -LiteralPath $configPath)) {
  throw "[preflight] Missing $configPath. Run scripts/bootstrap-local-model.ps1 -Model coder-1.5b"
}

$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
if (-not $config.llamaServerPath -or -not (Test-Path -LiteralPath $config.llamaServerPath)) {
  throw "[preflight] llama-server.exe not found: $($config.llamaServerPath)"
}
if (-not $config.modelPath -or -not (Test-Path -LiteralPath $config.modelPath)) {
  throw "[preflight] GGUF model not found: $($config.modelPath)"
}
Write-Host "[preflight] OK: model and server paths are valid"

if (Test-PortInUse -PortToCheck $Port) {
  Write-Warning "[preflight] port $Port is already in LISTEN state. Stop the existing process or use a different port."
} else {
  Write-Host "[preflight] OK: port $Port appears available"
}

$bridgePath = 'integrations\moa\synthesize_bridge.py'
if (-not (Test-Path -LiteralPath $bridgePath)) {
  throw "[preflight] missing bridge file: $bridgePath"
}
Write-Host '[preflight] running MoA bridge self-test'
Run-PythonCommand -PythonCmd $pythonCmd -Args @($bridgePath, '--self-test')

Write-Host '[preflight] PASS: demo environment is ready'
Write-Host '[preflight] next steps:'
Write-Host '  1) ./scripts/moa-winning-demo.ps1'
Write-Host '  2) pnpm desktop:tauri'
Write-Host '  3) follow docs/competition-demo-runbook.md'
