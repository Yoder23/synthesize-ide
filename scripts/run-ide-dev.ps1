param(
  [switch]$FrontendOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repo = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')
Set-Location $repo

$vsdev = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
if (-not (Test-Path -LiteralPath $vsdev)) {
  throw "Visual Studio Build Tools developer command file was not found: $vsdev"
}

$command = if ($FrontendOnly) { 'pnpm desktop:dev' } else { 'pnpm desktop:tauri' }

& cmd.exe /d /s /c "call `"$vsdev`" -arch=x64 -host_arch=x64 && set `"PATH=C:\Program Files\nodejs;%APPDATA%\npm;%USERPROFILE%\.cargo\bin;%PATH%`" && pnpm -r --filter `"./packages/**`" build && $command"
