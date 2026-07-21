param(
  [string]$EndpointUrl = 'http://127.0.0.1:8080/v1',
  [string]$Model = '',
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

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')
$configPath = Join-Path $repoRoot '.synthesize-runtime\local-model.json'
if (-not $Model -and (Test-Path -LiteralPath $configPath)) {
  $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
  $Model = Split-Path -Leaf $config.modelPath
}
if (-not $Model) {
  $Model = 'local-gguf'
}

$body = @{
  model = $Model
  temperature = 0.1
  max_tokens = 900
  stream = $false
  response_format = @{ type = 'json_object' }
  messages = @(
    @{
      role = 'system'
      content = 'You are Synthesize IDE local coding agent. Return only strict JSON. Do not use markdown fences. The only allowed operation types are propose_patch, run_command, report, ask_user, and final_report.'
    },
    @{
      role = 'user'
      content = @'
Create a Synthesize typed operation for this task.

Current file: src/auth/refresh.ts
beforeSha256=fixture-before-sha256

File content:
export function refreshToken() {
  throw new Error("not implemented");
}

Return exactly this shape and fill the patch text:
{
  "operations": [
    {
      "type": "propose_patch",
      "proposalId": "local-model-smoke",
      "summary": "Replace throwing refreshToken stub with a deterministic return value.",
      "files": [
        {
          "id": "local-model-smoke-file-001",
          "path": "src/auth/refresh.ts",
          "beforeSha256": "fixture-before-sha256",
          "patch": "diff --git a/src/auth/refresh.ts b/src/auth/refresh.ts\n--- a/src/auth/refresh.ts\n+++ b/src/auth/refresh.ts\n@@ -1,3 +1,3 @@\n export function refreshToken() {\n-  throw new Error(\"not implemented\");\n+  return \"refreshed\";\n }\n"
        }
      ],
      "riskNotes": ["Low-risk fixture patch for local model smoke test."],
      "suggestedCommands": [
        {
          "type": "run_command",
          "argv": ["pnpm", "test", "auth"],
          "cwd": ".",
          "reason": "Verify auth refresh behavior.",
          "expectedOutcome": "Auth tests pass.",
          "requiresNetwork": false,
          "mayModifyFiles": false
        }
      ]
    }
  ]
}
'@
    }
  )
} | ConvertTo-Json -Depth 8

Write-Host "[smoke] calling $EndpointUrl/chat/completions with model $Model"
$response = Invoke-RestMethod -Uri "$($EndpointUrl.TrimEnd('/'))/chat/completions" -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 180
$content = [string]$response.choices[0].message.content
Write-Host '[smoke] model response:'
Write-Host $content

$jsonText = $content.Trim()
if ($jsonText -match '(?s)^```(?:json)?\s*(.*?)\s*```$') {
  $jsonText = $Matches[1].Trim()
}
$parsed = $jsonText | ConvertFrom-Json
if (-not $parsed.operations -or $parsed.operations.Count -lt 1) {
  throw 'Model response did not contain operations.'
}
foreach ($op in @($parsed.operations)) {
  if ($op.type -notin @('propose_patch', 'run_command', 'report', 'ask_user', 'final_report')) {
    throw "Model emitted unsupported Synthesize operation type: $($op.type)"
  }
}

$bridge = Join-Path $repoRoot 'integrations\moa\synthesize_bridge.py'
if (-not (Test-Path -LiteralPath $bridge)) {
  throw "MoA bridge not found: $bridge"
}
$pythonCmd = Resolve-PythonExecutable $Python
foreach ($op in @($parsed.operations)) {
  $request = @{ command = 'evaluate_operation'; operation = $op } | ConvertTo-Json -Depth 10 -Compress
  if ($pythonCmd -eq 'py -3') {
    $decision = $request | & py -3 $bridge
  } else {
    $decision = $request | & $pythonCmd $bridge
  }
  Write-Host "[smoke] MoA decision: $decision"
  $json = $decision | ConvertFrom-Json
  if (-not $json.ok) {
    throw "MoA bridge failed: $decision"
  }
  if ($op.type -eq 'propose_patch' -and -not $json.approved) {
    throw "MoA rejected model patch proposal: $decision"
  }
}

Write-Host '[smoke] PASS: real local model produced Synthesize operations and MoA accepted the low-risk action.'
