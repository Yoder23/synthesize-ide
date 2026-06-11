# Synthesize IDE submission readiness

## Recommended track

**Creative Apps**

Synthesize IDE is a local-first AI coding workbench that makes agentic coding observable, interruptible, reversible, and auditable.

## One-sentence pitch

Synthesize IDE lets local AI models plan and propose code changes, but only a trusted backend can validate, approve, apply, roll back, and audit those actions.

## What changed in this package

- Added **MoA Action Planner** as a local agent profile.
- Added a visible **MoA Action Chat** mode with plan/action trace rendering.
- Added docs for MoA Action Mode.
- Updated the release gate to regenerate a placeholder-only lockfile before frozen install.
- Added `scripts/hydrate-lockfile.sh` for local dependency lockfile hydration.

## Required local validation before final upload

This archive cannot include a fully hydrated `pnpm-lock.yaml` unless dependencies are resolved in a network-enabled pnpm environment. Before final submission, run:

```bash
corepack enable
corepack prepare pnpm@9.15.0 --activate
./scripts/hydrate-lockfile.sh
pnpm typecheck
pnpm build
pnpm test
```

If Rust/Tauri is installed, also run:

```bash
cargo check --workspace
cargo test --workspace
./scripts/release-check.sh
```

Commit the generated `pnpm-lock.yaml` after `./scripts/hydrate-lockfile.sh`.

## Demo checklist

- Open fixture repo.
- Show local runtime/fake runtime.
- Select MoA Action Planner.
- Show context visibility and prompt hash.
- Ask for a repair.
- Show plan/action trace.
- Show typed operations and diff queue.
- Validate/approve/apply with checkpoint.
- Run tests.
- Show unsafe command block.
- Show audit log.
