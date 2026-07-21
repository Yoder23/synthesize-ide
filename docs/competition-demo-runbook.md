# Competition Demo Runbook

## Winning Thesis

Synthesize is not another coding chatbot. It is a local agent cockpit.

The model is useful but untrusted. It can propose typed operations, but it cannot touch the repo. MoA and the Synthesize host decide what becomes an action, record every transition, checkpoint before mutation, and prove unsafe actions are blocked.

## Setup Once

```powershell
./scripts/bootstrap-local-model.ps1 -Model coder-1.5b
./scripts/demo-preflight.ps1
```

This downloads Qwen2.5 Coder 1.5B GGUF and a local llama.cpp server binary into `.synthesize-runtime/`. No Ollama, no cloud account.

## Five-Minute Demo

### 0:00 - 0:30: Open With The Differentiator

Say:

> Most demos show an agent changing code. Synthesize shows why the action was allowed, what exact context produced it, how it was checkpointed, how to roll it back, and what was blocked. The model proposes. The system governs.

### 0:30 - 1:20: Run The Live Agent Loop

```powershell
./scripts/moa-winning-demo.ps1
```

What appears:

- The local GGUF model emits a Synthesize `propose_patch` operation.
- MoA approves the low-risk action.
- The trusted host validates path and `beforeSha256`.
- The host applies the exact model diff.
- Rollback restores the original file.
- MoA blocks a high-risk multi-file action.

### 1:20 - 2:20: Show Mission Control

Open:

```text
.synthesize-runtime/winning-demo/MISSION_CONTROL.html
```

Point to four cards:

- `Local Model`: Qwen2.5 Coder running through local llama.cpp.
- `Safe Action`: approved typed operation.
- `Unsafe Action`: blocked with MoA reason.
- `Audit Events`: JSONL flight recorder.

Also point to the in-app Mission Control scorecard counters:

- Novelty score
- Evidence completeness
- Blocked unsafe actions
- P50 model response latency

Say:

> This is the agent flight recorder. A judge can inspect the model output, the gate decision, the checkpoint, the rollback, and the blocked counterexample.

### 2:20 - 3:20: Show The Audit Trail

Open:

```text
.synthesize-runtime/winning-demo/audit.jsonl
```

Call out these event names:

- `context.bundle_created`
- `runtime.request_completed`
- `operation.parse_succeeded`
- `moa.gate_decision`
- `patch.validated`
- `patch.applied`
- `moa.high_risk_blocked`
- `patch.rolled_back`

Say:

> This is the difference between an agent and an accountable agent. The system can replay who proposed what, what was approved, what was changed, and how it was reversed.

### 3:20 - 4:10: Show The Report

Open:

```text
.synthesize-runtime/winning-demo/PRESENTATION_REPORT.md
```

Show:

- Raw model output.
- Applied file before rollback.
- Final file after rollback.
- Blocked unsafe action reason.

Say:

> The demo is not a prompt trick. The model output is captured raw. The patch applied is the exact diff it proposed. Then rollback restores the original file.

### 4:10 - 5:00: Close With The Claim

Say:

> Coding with a local model is table stakes. Synthesize adds the missing action layer: typed operations, MoA gating, path/hash validation, checkpointed apply, rollback, and audit transparency. This is how local agents become trustworthy enough to act.

Add this compliance sentence during Q&A or closing if competition rules request AI tooling disclosure:

> Synthesize was built with significant assistance from GitHub Copilot, while architecture, governance policy, and release decisions were made by the project author.

## Optional Live Variation

Use a custom goal:

```powershell
./scripts/moa-winning-demo.ps1 -Goal "Repair refreshToken so tests can assert a successful auth refresh result."
```

The model still has to emit typed operations. The host still validates, gates, applies, blocks, and rolls back.

## Running The IDE

This machine must have Node/Corepack/pnpm and Rust/Cargo on PATH. Then:

```powershell
corepack enable
corepack prepare pnpm@9.15.0 --activate
pnpm install
pnpm desktop:tauri
```

In the IDE:

1. Open the fixture repo at `.synthesize-runtime/winning-demo/repo` or any clean demo repo.
2. Go to Local Agent Profile and select `MoA Action Planner`.
3. Go to Local Model Runtime Control.
4. Select `Managed llama.cpp`.
5. Use these paths from `.synthesize-runtime/local-model.json`:
   - `llamaServerPath`
   - `modelPath`
6. Start managed llama.cpp.
7. Health check the localhost endpoint.
8. Ask: `Repair refreshToken so auth refresh returns a stable success token instead of throwing.`
9. Show context visibility and prompt hash.
10. Show typed operation parsing.
11. Validate the diff.
12. Approve and apply.
13. Roll back.
14. Show the Session Log.

## Answer To "Is This Just NemoClaw?"

No. The core demo is not "we block dangerous actions." The core demo is an end-to-end governed action lifecycle:

- local model generation,
- typed operation protocol,
- MoA decision,
- backend path/hash validation,
- checkpointed mutation,
- rollback,
- append-only audit,
- explicit unsafe counterexample.

The novelty is the combination: local-first agent action with a visible, replayable authority boundary.
