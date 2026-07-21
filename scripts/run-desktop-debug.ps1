param(
  [string]$BinaryPath = '.\target\debug\synthesize-ide-desktop.exe'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Set-Location -LiteralPath $repo
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path

# A cargo-built debug binary is a Tauri development binary. It loads the
# frontend from Vite (1420), so start that server before launching the binary.
$vite = Start-Process -FilePath 'cmd.exe' -ArgumentList '/d', '/c', 'pnpm --filter synthesize-ide-desktop dev --host 127.0.0.1' -WorkingDirectory $repo -PassThru -WindowStyle Hidden
try {
  $ready = $false
  1..40 | ForEach-Object {
    Start-Sleep -Milliseconds 250
    if (Test-NetConnection -ComputerName 127.0.0.1 -Port 1420 -InformationLevel Quiet -WarningAction SilentlyContinue) {
      $ready = $true
      return
    }
  }
  if (-not $ready) { throw 'Vite did not start listening on http://127.0.0.1:1420.' }

  & $binary
  if ($LASTEXITCODE -ne 0) { throw "Synthesize exited with code $LASTEXITCODE." }
}
finally {
  if (Get-Process -Id $vite.Id -ErrorAction SilentlyContinue) {
    & taskkill.exe /PID $vite.Id /T /F 2>$null | Out-Null
  }
}
