param(
  [ValidateSet('coder-0.6b', 'coder-1.7b', 'coder-8b')]
  [string]$Model = 'coder-1.7b',
  [string]$InstallRoot = '',
  [switch]$SkipLlamaCpp
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')
if (-not $InstallRoot) {
  $InstallRoot = Join-Path $repoRoot '.synthesize-runtime'
}

$modelsDir = Join-Path $InstallRoot 'models'
$llamaDir = Join-Path $InstallRoot 'llamacpp'
$downloadsDir = Join-Path $InstallRoot 'downloads'
New-Item -ItemType Directory -Force -Path $modelsDir, $llamaDir, $downloadsDir | Out-Null

$modelCatalog = @{
  'coder-0.6b' = @{
    Name = 'Qwen3 Coder 0.6B Instruct Q4_K_M'
    SkillTier = 'fast'
    FileName = 'qwen3-coder-0.6b-instruct-q4_k_m.gguf'
    Url = 'https://huggingface.co/Qwen/Qwen3-Coder-0.6B-Instruct-GGUF/resolve/main/qwen3-coder-0.6b-instruct-q4_k_m.gguf'
    RecommendedRamGb = 4
    Port = 8081
    CtxSize = 32768
  }
  'coder-1.7b' = @{
    Name = 'Qwen3 Coder 1.7B Instruct Q4_K_M'
    SkillTier = 'balanced'
    FileName = 'qwen3-coder-1.7b-instruct-q4_k_m.gguf'
    Url = 'https://huggingface.co/Qwen/Qwen3-Coder-1.7B-Instruct-GGUF/resolve/main/qwen3-coder-1.7b-instruct-q4_k_m.gguf'
    RecommendedRamGb = 6
    Port = 8080
    CtxSize = 32768
  }
  'coder-8b' = @{
    Name = 'Qwen3 Coder 8B Instruct Q4_K_M'
    SkillTier = 'powerful'
    FileName = 'qwen3-coder-8b-instruct-q4_k_m.gguf'
    Url = 'https://huggingface.co/Qwen/Qwen3-Coder-8B-Instruct-GGUF/resolve/main/qwen3-coder-8b-instruct-q4_k_m.gguf'
    RecommendedRamGb = 14
    Port = 8082
    CtxSize = 65536
  }
}

$catalog = $modelCatalog[$Model]

Write-Host ''
Write-Host '========================================================'
Write-Host '  Synthesize IDE - Qwen3 Skill Agent Bootstrap'
Write-Host '========================================================'
Write-Host ("  Model : {0}" -f $catalog.Name)
Write-Host ("  Tier  : {0}" -f $catalog.SkillTier)
Write-Host ("  RAM   : {0} GB recommended" -f $catalog.RecommendedRamGb)
Write-Host ("  Root  : {0}" -f $InstallRoot)
Write-Host '========================================================'
Write-Host ''

function Download-File([string]$Url, [string]$OutFile) {
  $fileName = Split-Path -Leaf $OutFile
  if (Test-Path -LiteralPath $OutFile) {
    Write-Host ("[bootstrap] already present: {0}" -f $fileName)
    return
  }
  Write-Host ("[bootstrap] downloading {0} ..." -f $fileName)
  Write-Host ("  URL: {0}" -f $Url)
  $ProgressPreference = 'SilentlyContinue'
  Invoke-WebRequest -Uri $Url -OutFile $OutFile
  $ProgressPreference = 'Continue'
  $sizeGb = [math]::Round((Get-Item $OutFile).Length / 1GB, 2)
  Write-Host ("[bootstrap] downloaded {0} GB -> {1}" -f $sizeGb, $fileName)
}

function Find-LlamaServer([string]$Root) {
  Get-ChildItem -LiteralPath $Root -Recurse -Filter 'llama-server.exe' -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName
}

function Install-LlamaCpp([string]$TargetDir, [string]$DownloadDir) {
  $existing = Find-LlamaServer $TargetDir
  if ($existing) {
    Write-Host ("[bootstrap] llama-server already present: {0}" -f $existing)
    return $existing
  }

  Write-Host '[bootstrap] fetching latest llama.cpp Windows release metadata'
  $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/ggml-org/llama.cpp/releases/latest'
  $assets = @($release.assets)
  $asset = $assets |
    Where-Object {
      $_.name -match 'win' -and $_.name -match 'zip' -and
      $_.name -match '(x64|x86_64)' -and
      $_.name -notmatch '(cuda|cublas|vulkan|kompute|hip|sycl)'
    } |
    Select-Object -First 1
  if (-not $asset) {
    $asset = $assets | Where-Object { $_.name -match 'win' -and $_.name -match 'zip' } | Select-Object -First 1
  }
  if (-not $asset) {
    throw 'Could not find a Windows llama.cpp zip asset in the latest GitHub release.'
  }

  $zip = Join-Path $DownloadDir $asset.name
  Download-File $asset.browser_download_url $zip
  Write-Host ("[bootstrap] extracting {0} ..." -f $asset.name)
  Expand-Archive -LiteralPath $zip -DestinationPath $TargetDir -Force

  $server = Find-LlamaServer $TargetDir
  if (-not $server) {
    throw 'llama-server.exe not found after extraction.'
  }
  Write-Host ("[bootstrap] llama-server: {0}" -f $server)
  return $server
}

$modelPath = Join-Path $modelsDir $catalog.FileName
Download-File $catalog.Url $modelPath

$llamaServerPath = $null
if (-not $SkipLlamaCpp) {
  $llamaServerPath = Install-LlamaCpp $llamaDir $downloadsDir
} else {
  Write-Host '[bootstrap] SkipLlamaCpp set; not downloading llama.cpp.'
  $llamaServerPath = Find-LlamaServer $llamaDir
}

$launchScriptPath = Join-Path $InstallRoot ("start-qwen3-{0}.ps1" -f $Model)
if ($llamaServerPath) {
  $launchContent = @"
param([int]`$Port = $($catalog.Port), [int]`$CtxSize = $($catalog.CtxSize), [int]`$Threads = 4, [int]`$GpuLayers = 0)
Set-StrictMode -Version Latest
`$ErrorActionPreference = 'Stop'
Write-Host "Starting $($catalog.Name) on port `$Port ..."
& '$llamaServerPath' `
  --model '$modelPath' `
  --host 127.0.0.1 `
  --port `$Port `
  --ctx-size `$CtxSize `
  --threads `$Threads `
  --n-gpu-layers `$GpuLayers `
  --no-webui
"@
  $launchContent | Set-Content -LiteralPath $launchScriptPath -Encoding UTF8
  Write-Host ("[bootstrap] launch script: {0}" -f $launchScriptPath)
}

# qwen-specific config
$qwenConfigPath = Join-Path $InstallRoot ("qwen3-{0}-config.json" -f $Model)
$qwenConfig = [ordered]@{
  synthesize_runtime_config = 'v1'
  model_name = $catalog.Name
  skill_tier = $catalog.SkillTier
  model_path = $modelPath
  llama_server_path = $llamaServerPath
  endpoint_url = ("http://127.0.0.1:{0}/v1" -f $catalog.Port)
  ctx_size = $catalog.CtxSize
}
$qwenConfig | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $qwenConfigPath -Encoding UTF8
Write-Host ("[bootstrap] qwen config: {0}" -f $qwenConfigPath)

# IDE-compatible local-model.json sync (used by start-local-model.ps1 and smoke script)
$modelHash = (Get-FileHash -LiteralPath $modelPath -Algorithm SHA256).Hash.ToLowerInvariant()
$localConfigPath = Join-Path $InstallRoot 'local-model.json'
$localConfig = [ordered]@{
  model = $Model
  displayName = $catalog.Name
  modelId = "Qwen/$($catalog.Name -replace ' ', '-')"
  modelPath = $modelPath
  modelSha256 = $modelHash
  llamaServerPath = $llamaServerPath
  endpointUrl = ("http://127.0.0.1:{0}/v1" -f $catalog.Port)
  recommendedRamGb = $catalog.RecommendedRamGb
  installedAt = (Get-Date).ToUniversalTime().ToString('o')
}
$localConfig | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $localConfigPath -Encoding UTF8
Write-Host ("[bootstrap] synced IDE local-model config: {0}" -f $localConfigPath)

Write-Host ''
Write-Host 'Bootstrap complete.'
Write-Host ("Start server with: powershell -ExecutionPolicy Bypass -File {0}" -f $launchScriptPath)
Write-Host ("Then smoke test: powershell -ExecutionPolicy Bypass -File scripts/local-model-smoke.ps1 -EndpointUrl http://127.0.0.1:{0}/v1 -Model {1}" -f $catalog.Port, $catalog.FileName)
