param(
  [string]$Python = '',
  [string]$Goal = 'Repair refreshToken so the auth flow returns a stable success token instead of throwing.'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

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

Set-Location $PSScriptRoot\..
$pythonCmd = Resolve-PythonExecutable $Python
if ($pythonCmd -eq 'py -3') {
  & py -3 scripts\moa_winning_demo.py --goal $Goal
} else {
  & $pythonCmd scripts\moa_winning_demo.py --goal $Goal
}
