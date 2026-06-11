Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot\..

$root = Get-Location
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$outDir = Join-Path $root 'dist\submission'
$stageDir = Join-Path $outDir "Synthesize-IDE-$stamp"
$zipPath = "$stageDir.zip"

if (Test-Path $stageDir) { Remove-Item $stageDir -Recurse -Force }
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
New-Item -ItemType Directory -Path $stageDir -Force | Out-Null

$exclude = @(
  '\\.git($|\\)',
  '\\node_modules($|\\)',
  '\\target($|\\)',
  '\\.turbo($|\\)',
  '\\.next($|\\)',
  '\\dist($|\\)',
  '\\integrations\\moa\\.pytest_cache($|\\)'
)

Get-ChildItem -Recurse -Force | ForEach-Object {
  $full = $_.FullName
  $rel = $full.Substring($root.Path.Length).TrimStart('\\')
  if ([string]::IsNullOrWhiteSpace($rel)) { return }
  foreach ($pattern in $exclude) {
    if ($full -match $pattern) { return }
  }
  $dest = Join-Path $stageDir $rel
  if ($_.PSIsContainer) {
    New-Item -ItemType Directory -Path $dest -Force | Out-Null
  } else {
    $destDir = Split-Path -Parent $dest
    if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Path $destDir -Force | Out-Null }
    Copy-Item $full $dest -Force
  }
}

Compress-Archive -Path "$stageDir\*" -DestinationPath $zipPath -CompressionLevel Optimal

Write-Host "Submission bundle created: $zipPath"