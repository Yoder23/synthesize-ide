# Synthesize IDE - Microsoft Agent Competition Packet

## Title

Synthesize IDE: Local AI coding with MoA-governed action safety

## One-line pitch

Synthesize IDE turns a local coding model into an auditable action-taking agent: the model proposes, MoA gates, Synthesize validates, checkpoints, applies, blocks unsafe actions, and records every step.

## AI tooling disclosure

This project was built with significant assistance from GitHub Copilot (GPT-5.3-Codex) for implementation support, refactoring, and documentation drafting. Final architecture, governance, and release decisions were made by the project author.

## Category

Creative Apps

## Why this is an agent system

- The model plans and proposes typed operations.
- The backend enforces lifecycle control over patch proposals.
- Execution authority is separated from generation.
- Safety and rollback are first-class runtime behaviors.

## Core architecture claim

The model is outside the trusted action boundary. It can propose operations, but cannot directly modify files or execute shell commands.

Trust and authority remain in the backend:

- Context construction
- Patch validation (path/hash/policy)
- Approval state machine
- Transactional apply and rollback
- Governed task execution
- Audit persistence

See `docs/submission-architecture.md` and `docs/moa-bridge.md`.

## What is bundled in this repo

- Synthesize IDE monorepo (`apps/*`, `packages/*`)
- Bundled MoA integration at `integrations/moa`
- MoA bridge process at `integrations/moa/synthesize_bridge.py`
- Bridge tests at `integrations/moa/tests/test_synthesize_bridge.py`

## Live proof script (2 minutes)

Run the no-Ollama local model action proof:

```powershell
./scripts/bootstrap-local-model.ps1 -Model coder-1.5b
./scripts/demo-preflight.ps1
./scripts/moa-winning-demo.ps1
```

This starts a local llama.cpp server for Qwen2.5 Coder 1.5B GGUF, asks the model for a Synthesize typed patch operation, gates it through MoA, validates path and before-hash, applies it with a checkpoint, writes an audit log, then proves a high-risk action is blocked before execution.

Generated artifacts:

- `.synthesize-runtime/winning-demo/audit.jsonl`
- `.synthesize-runtime/winning-demo/PRESENTATION_REPORT.md`
- `.synthesize-runtime/winning-demo/MISSION_CONTROL.html`
- `.synthesize-runtime/winning-demo/repo/src/auth/refresh.ts`
- `.synthesize-runtime/winning-demo/checkpoint/src/auth/refresh.ts`

## GUI demo script (5-7 minutes)

1. Open Synthesize IDE and a sample repo.
2. Select **MoA Action Planner** profile.
3. Ask for a small code repair.
4. Show typed operations and plan/action trace in chat.
5. Show diff queue and backend validation/approval flow.
6. Apply patch with checkpoint.
7. Run governed test command.
8. Show audit log and optional rollback.
9. Show unsafe command blocked in Personal Terminal policy path.

## Validation commands

```powershell
./scripts/submission-check.ps1
```

This runs:

- `cargo check --workspace --exclude synthesize-ide-desktop`
- `cargo test --workspace --exclude synthesize-ide-desktop`
- `pnpm install --no-frozen-lockfile`
- `pnpm typecheck`
- `pnpm build`
- `pnpm test`
- `py -3 integrations/moa/synthesize_bridge.py --self-test` (or `python ...`)
- `py -3 integrations/moa/verify_moa.py` (or `python ...`)
- `py -3 -m pytest integrations/moa/tests -q` (or `python -m pytest ...`)

`submission-check.ps1` resolves Python automatically via `py -3` or `python` unless `-Python` is provided.

## Packaging

Create a submission archive:

```powershell
./scripts/submission-bundle.ps1
```

Output archive is written under `dist/submission/`.

## Reviewer notes

- This project is local-first and works with self-hosted model endpoints.
- OpenAI-compatible wire protocol support does not require OpenAI service usage.
- The model never receives direct execution authority in this architecture.
