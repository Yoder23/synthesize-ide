# Synthesize v18 Personal-Production Release Candidate

V18 focuses on the practical daily loop for a personal local-model IDE: select code, ask the local agent, apply governed patches, run governed tests/builds, feed task output back into the agent, and commit when satisfied.

## Added

- Inline selection actions in the editor toolbar:
  - Explain selection
  - Fix selection
  - Write tests for selection
- Command Palette entries for selection-based agent actions.
- Agent draft handoff: selection/task prompts populate Agent Chat without bypassing backend context building.
- Governed task output → agent repair loop:
  - after a task run, the output can be sent back to Agent Chat as a repair prompt;
  - output is truncated before becoming prompt text;
  - the agent still returns typed operations only.
- Governed task history and rerun UX.
- Explicit cancel-state messaging for the current synchronous restricted runner.
- Ready-to-Work panel showing repo/runtime/profile/dirty-state readiness.
- Personal-production docs and manual QA script.

## Preserved

- Backend-owned context bundles and model calls.
- Backend-owned patch validation/approval/apply/rollback.
- Backend-owned task snapshots and approvals.
- Agent-suggested commands remain classification-only.
- Governed tasks are the only executable code path and remain argv-only, timeout-bounded, env-scrubbed, and audited.

## Still required before calling production-ready

- Commit `pnpm-lock.yaml`.
- Run `./scripts/release-check.sh` successfully.
- Smoke-test fake runtime, managed llama.cpp, manual local endpoint, real local-model patch, apply/rollback, and governed task repair loop.
