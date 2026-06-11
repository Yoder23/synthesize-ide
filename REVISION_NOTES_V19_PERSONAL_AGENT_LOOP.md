# Synthesize v19 Personal Agent Loop

Goal: make Synthesize more useful as a personal local/self-hosted alternative to Copilot-in-VS Code without turning it into an unsafe free shell.

## Added

- Personal Terminal panel inside the editor workbench.
- User-entered repo-local commands are approved by the backend before execution.
- Commands run as argv directly, not through a shell, so there is no shell interpolation path.
- CWD is resolved through RepoGuard and must remain inside the opened repo.
- Execution reuses the existing audited, timeout-bounded, env-scrubbed task runner path.
- Terminal output can be fed directly back into the agent as a repair prompt.
- Backend command `approve_personal_command` persists a local-user approval before `run_approved_task` can execute it.
- Context prompt now tells the model that it may suggest commands, but only the local user can approve/run them.

## Still intentionally constrained

- Network/destructive/blocked commands remain refused by CommandPolicy.
- Commands are allowlist-based and do not run through `bash`, `sh`, `zsh`, `pwsh`, or `cmd`.
- There is still no OS-level network sandbox.
- Execution is synchronous and bounded by the existing 120-second timeout.

## Intended personal workflow

1. Open a repo.
2. Ask the local/self-hosted model for a change.
3. Review the typed operation and diff.
4. Validate, approve, and apply the patch.
5. Run `pnpm test`, `cargo test`, `go test ./...`, `pytest`, etc. from Personal Terminal.
6. Feed failing output back to the agent.
7. Repeat until green.
