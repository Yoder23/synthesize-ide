param(
  [ValidateSet('smoke', 'coder-1.5b')]
  [string]$Model = 'smoke',
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
  'smoke' = @{
    Name = 'Qwen2.5 Coder 0.5B Instruct Q4_K_M'
    ModelId = 'Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF'
    FileName = 'qwen2.5-coder-0.5b-instruct-q4_k_m.gguf'
    Url = 'https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf'
    RecommendedRamGb = 4
  }
  'coder-1.5b' = @{
    Name = 'Qwen2.5 Coder 1.5B Instruct Q4_K_M'
    ModelId = 'Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF'
    FileName = 'qwen2.5-coder-1.5b-instruct-q4_k_m.gguf'
    Url = 'https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf'
    RecommendedRamGb = 6
  }
}

function Download-File([string]$Url, [string]$OutFile) {
  if (Test-Path -LiteralPath $OutFile) {
    Write-Host "[bootstrap] already present: $OutFile"
    return
  }
  Write-Host "[bootstrap] downloading $Url"
  Invoke-WebRequest -Uri $Url -OutFile $OutFile
}

function Find-LlamaServer([string]$Root) {
  Get-ChildItem -LiteralPath $Root -Recurse -Filter 'llama-server.exe' -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName
}

function Install-LlamaCpp([string]$TargetDir, [string]$DownloadDir) {
  $existing = Find-LlamaServer $TargetDir
  if ($existing) {
    Write-Host "[bootstrap] llama-server already present: $existing"
    return $existing
  }

  Write-Host '[bootstrap] resolving latest llama.cpp Windows CPU release asset'
  $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/ggml-org/llama.cpp/releases/latest'
  $assets = @($release.assets)
  $asset = $assets |
    Where-Object {
      $_.name -match 'win' -and
      $_.name -match 'zip' -and
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
  Write-Host "[bootstrap] extracting $($asset.name)"
  Expand-Archive -LiteralPath $zip -DestinationPath $TargetDir -Force

  $server = Find-LlamaServer $TargetDir
  if (-not $server) {
    throw "Downloaded llama.cpp archive did not contain llama-server.exe: $($asset.name)"
  }
  return $server
}

$selected = $modelCatalog[$Model]
$modelPath = Join-Path $modelsDir $selected.FileName
Download-File $selected.Url $modelPath

Write-Host '[bootstrap] calculating model sha256'
$modelHash = (Get-FileHash -LiteralPath $modelPath -Algorithm SHA256).Hash.ToLowerInvariant()
$existingConfigPath = Join-Path $InstallRoot 'local-model.json'
$existingServer = ''
if (Test-Path -LiteralPath $existingConfigPath) {
  try {
    $existing = Get-Content -LiteralPath $existingConfigPath -Raw | ConvertFrom-Json
    if ($existing.llamaServerPath -and (Test-Path -LiteralPath $existing.llamaServerPath)) {
      $existingServer = [string]$existing.llamaServerPath
    }
  } catch {
    $existingServer = ''
  }
}
$llamaServer = if ($SkipLlamaCpp) { $existingServer } else { Install-LlamaCpp $llamaDir $downloadsDir }

$config = [ordered]@{
  model = $Model
  displayName = $selected.Name
  modelId = $selected.ModelId
  modelPath = $modelPath
  modelSha256 = $modelHash
  llamaServerPath = $llamaServer
  endpointUrl = 'http://127.0.0.1:8080/v1'
  recommendedRamGb = $selected.RecommendedRamGb
  installedAt = (Get-Date).ToUniversalTime().ToString('o')
}
$configPath = $existingConfigPath
$config | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $configPath -Encoding UTF8

Write-Host '[bootstrap] local model bootstrap complete'
Write-Host "[bootstrap] config: $configPath"
Write-Host "[bootstrap] model:  $modelPath"
if ($llamaServer) {
  Write-Host "[bootstrap] server: $llamaServer"
}
