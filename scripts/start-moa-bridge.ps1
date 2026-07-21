param(
	[string]$Python = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

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

Set-Location $PSScriptRoot\..
$bridgePath = 'integrations\moa\synthesize_bridge.py'
if (-not (Test-Path -LiteralPath $bridgePath)) {
	throw "MoA bridge not found: $bridgePath"
}

$pythonCmd = Resolve-PythonExecutable $Python
if ($pythonCmd -eq 'py -3') {
	& py -3 $bridgePath
} else {
	& $pythonCmd $bridgePath
}