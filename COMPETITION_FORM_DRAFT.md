# Microsoft Agent Competition - Form Draft

## Project name

Synthesize IDE

## Category / track

Creative Apps

## Short pitch

Synthesize IDE is a local-first agentic coding workbench where models propose typed operations, but only a trusted backend can validate, approve, apply, roll back, and audit changes.

## Problem

Most coding agents blur planning and execution. That makes behavior hard to inspect, hard to interrupt, and hard to roll back safely.

## Solution

Synthesize IDE separates proposal from authority:

- The model emits typed operations only.
- The backend owns context construction, validation, approval state, apply/rollback, and audit.
- MoA governance is bundled for explicit action risk evaluation.

## Why this is agentic

- Multi-step planning through typed action proposals.
- Governed decision points before execution.
- Iterative repair loops using task outputs and contextual recall.
- Persistent audit trail for explainability and accountability.

## Key technical differentiators

- Typed operation protocol between model and system actor.
- Backend-owned trust boundary and immutable lifecycle checkpoints.
- Built-in rollback path and session audit log.
- Local runtime support (self-hosted OpenAI-compatible endpoints).
- Bundled MoA bridge (`integrations/moa/synthesize_bridge.py`).

## Safety and governance

- Model output is untrusted by design.
- No direct shell/file authority for the model.
- Patch and command pathways are policy-gated.
- Action evaluation and risk metadata are visible and auditable.

## Demo flow (5-7 minutes)

1. Open Synthesize IDE and a sample repo.
2. Select **MoA Action Planner** profile.
3. Ask for a small repair.
4. Show typed operations and action trace.
5. Validate, approve, and apply a proposal.
6. Run governed checks.
7. Show audit and rollback.

## Validation commands

```powershell
./scripts/submission-check.ps1
```

Fallback for constrained local environments:

```powershell
./scripts/submission-check.ps1 -SkipRust -SkipFrontend
```

## Packaging command

```powershell
./scripts/submission-bundle.ps1
```

Output archive is written under `dist/submission/`.

## Repository URL

https://github.com/Yoder23/synthesize-ide