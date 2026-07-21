# Baseline validation

Baseline date: 2026-07-19. The worktree already contained user-owned edits and untracked competition/skill-agent files before this upgrade.

| Command | Outcome | Baseline observation |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Failed | Pre-existing formatting differences across the Tauri backend and multiple crates. |
| `cargo check --workspace` | Passed | Completed with dead-code warnings in patch-engine and the desktop. |
| `cargo test --workspace` | Passed | 65 passed, 0 failed, 1 ignored local-Ollama test. |
| `pnpm typecheck` | Environment failure | PowerShell execution policy blocked `pnpm.ps1`. |
| `pnpm.cmd typecheck` | Passed | All eight applicable workspace projects passed. |
| `pnpm.cmd test` | Passed | Existing test scripts were TypeScript compilation checks. |
| `pnpm.cmd build` | Passed | All packages and the Vite production bundle built. |
| `scripts/submission-check.ps1` | False-positive pass | Rust, build/typecheck/test, MoA verification, and 95 Python tests passed. `pnpm install` had a registry fetch failure, but the script continued and printed PASS because native exit codes were not checked. |

The formatting and release-script defects predate the outcome-studio implementation and are included in hardening work. Network-dependent and real-model smoke tests remain environmental/manual checks.

