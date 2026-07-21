param(
  [string]$ConfigPath = '',
  [int]$Port = 8080,
  [int]$CtxSize = 1024,
  [int]$GpuLayers = 0,
  [string]$Alias = 'local-gguf'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')
if (-not $ConfigPath) {
  $ConfigPath = Join-Path $repoRoot '.synthesize-runtime\local-model.json'
}
if (-not (Test-Path -LiteralPath $ConfigPath)) {
  throw "Local model config not found. Run scripts\bootstrap-local-model.ps1 first. Missing: $ConfigPath"
}

$config = Get-Content -LiteralPath $ConfigPath -Raw | ConvertFrom-Json
if (-not (Test-Path -LiteralPath $config.llamaServerPath)) {
  throw "llama-server.exe not found: $($config.llamaServerPath)"
}
if (-not (Test-Path -LiteralPath $config.modelPath)) {
  throw "GGUF model not found: $($config.modelPath)"
}

$endpoint = "http://127.0.0.1:$Port/v1"
Write-Host "[local-model] starting llama.cpp server"
Write-Host "[local-model] endpoint: $endpoint"
Write-Host "[local-model] model:    $($config.modelPath)"

& $config.llamaServerPath `
  --model $config.modelPath `
  --host 127.0.0.1 `
  --port $Port `
  --ctx-size $CtxSize `
  --n-gpu-layers $GpuLayers `
  --threads 4 `
  --no-webui `
  --alias $Alias
