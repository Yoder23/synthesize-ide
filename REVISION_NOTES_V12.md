# Synthesize v12 production-tightening pass

V12 is a release-candidate hardening pass focused on production discipline without broadening product scope.

## Changes

- Added `pnpm-workspace.yaml` so pnpm workspaces are explicit and reproducible.
- Added `scripts/release-check.sh` as the single local release gate:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `pnpm install --frozen-lockfile`
  - `pnpm typecheck`
  - `pnpm build`
  - `pnpm test`
- Added backend Agent Profile operation-policy enforcement for patch validation:
  - `local-patcher` and `fake-demo` may validate `propose_patch` operations.
  - `local-planner` and `local-reviewer` cannot validate/apply patch proposals; they are report/planning/review profiles.
  - command operations remain classification-only and cannot enter the patch lifecycle.
- Updated the frontend validation request to include the selected `agent_profile_id`.
- Added Rust tests for Agent Profile patch-validation policy.
- Updated README and known limitations to describe v12 as a release-candidate production-tightening pass.

## Still required before a production release

This environment cannot run Cargo or pnpm, so v12 still requires real build verification in a developer environment:

```bash
./scripts/release-check.sh
```

Then generate/commit `pnpm-lock.yaml` and switch CI to frozen-lockfile mode if the lockfile is not already present.

## Scope deliberately not changed

- No command execution.
- No model downloader.
- No cloud provider presets.
- No autonomous background tasks.
- No autocomplete.
- No OS-level sandbox claim.
