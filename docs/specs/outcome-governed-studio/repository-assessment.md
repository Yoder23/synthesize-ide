# Repository assessment

Assessment date: 2026-07-19

## Current architecture

Synthesize is a pnpm/TypeScript workspace plus a Rust workspace. The React/Tauri desktop application is the composition root. TypeScript packages provide shared types, operation parsing, prompts, context helpers, runtime adapters, model metadata, and skill definitions. Rust crates provide RepoGuard, command policy, patch validation/application/checkpoints, runtime abstractions, audit persistence, and bounded process execution.

The trusted effect boundary is real: runtime output is parsed into typed operations; patch proposals are persisted with hashes; approval, apply, checkpoint, rollback, task execution, endpoint egress approval, and audit events are backend-owned. Existing Assist Mode must continue to use that path.

## Located implementation paths

| Concern | Current implementation |
| --- | --- |
| Context bundles | `build_context_bundle` and `load_context_bundle` in the Tauri backend |
| Runtime calls | `runtime_generate`, endpoint approval, runtime adapters, managed llama supervisor |
| Profile compilation | backend `build_synthesize_system_prompt`; TypeScript prompt compiler and AgentPanel |
| Typed operation parsing | `packages/agent-harness` and `crates/agent-protocol` |
| Patch persistence and governance | Tauri proposal repository helpers plus `patch-engine` |
| Tasks and commands | Tauri task snapshots, `command-guard`, `sandbox-runner` |
| Audit/session replay | `audit-log`, `list_audit_events`, SessionLog |
| UI command boundary | Tauri `invoke` calls from feature modules |

## Findings

1. `apps/desktop/src-tauri/src/main.rs` is a 4,000-line composition root containing domain, persistence, runtime, and command-adapter logic. New studio behavior must be extracted into testable crates/modules.
2. `App.tsx` is also broad. New product modes need a dedicated feature workspace rather than more inline state.
3. Persistence initially had no explicit ordered schema migrations. Existing tables and data must be retained while studio tables are added forward-only.
4. Agent roles and the skill queue are useful prior work, but do not implement authoritative initiatives, frozen specs, evidence gates, Dream mandates, worktree binding, or Pulse.
5. Fake Runtime covers the Assist patch loop but not deterministic role artifacts and orchestration verdict scenarios.
6. Context selection is backend-owned for Assist, while the TypeScript context helper remains minimal. Studio needs role-specific bundles, deterministic ordering, budgets, and redaction.
7. Rust formatting failed at baseline across pre-existing files. Compilation and tests passed.
8. The submission PowerShell script did not enforce native command exit codes, allowing a package-install failure to be followed by a false `PASS`.
9. Runtime cancellation reports itself as unimplemented; Studio scheduling therefore needs a persisted cancellation/pause policy rather than pretending blocking provider calls are preemptible.
10. No OS sandbox or complete network isolation exists. Dream safety must be described as backend path/command/worktree governance, not OS containment.

## Migration direction

The upgrade adds four focused Rust crates: an intent/evidence ledger, an orchestration core, a Pulse engine, and a worktree manager. The desktop gets a thin `studio` Tauri adapter and a modular Studio workspace. Existing Assist commands remain intact and are reused for governed patches and validation rather than replaced.

