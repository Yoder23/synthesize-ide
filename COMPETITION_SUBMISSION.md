# Synthesize IDE - Microsoft Agent Competition Packet

## Title

Synthesize IDE: Local AI coding with MoA-governed action safety

## One-line pitch

Synthesize IDE lets local AI models propose coding actions while a trusted MoA/Synthesize backend validates, approves, applies, rolls back, and audits every change.

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

## Demo script (5-7 minutes)

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

- `cargo check --workspace`
- `cargo test --workspace`
- `pnpm install --frozen-lockfile`
- `pnpm typecheck`
- `pnpm build`
- `pnpm test`
- `C:\Python310\python.exe integrations/moa/synthesize_bridge.py --self-test`
- `C:\Python310\python.exe -m pytest integrations/moa/tests/test_synthesize_bridge.py -q`

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