# Synthesize v19.1 — Personal Terminal hardening

This revision hardens the v19 Personal Terminal before treating Synthesize as personal-production ready.

## What changed

- Added a strict `personal_terminal_policy()` in `crates/command-guard`.
- Personal Terminal now uses explicit-rule-only classification.
- Removed the permissive allowlisted-program fallback for user-entered commands.
- Removed Personal Terminal UI checkboxes for `requiresNetwork` and `mayModifyFiles`; they are no longer treated as a safety input.
- Re-runs of approved Personal Terminal commands are reclassified with the strict personal policy.
- Expanded explicit Git policy:
  - allowed: `git status`, `git diff`, `git log`
  - blocked: `git push`, `git pull`, `git fetch`, `git checkout`, `git switch`, `git restore`, `git add`, `git commit`, `git merge`, `git rebase`, `git reset`, `git clean`, `git stash`
- Added explicit safe/read/test/build rules for:
  - `rg`, `ls`, `cat`
  - `npm test`, `npm run test|lint|build|typecheck`
  - `pnpm test`, `pnpm run test|lint|build|typecheck`
  - `yarn test`, `yarn run test|lint|build|typecheck`
  - `cargo test`, `go test`, `pytest`, `dotnet test`
- Added command-guard tests for dangerous Git commands, strict fallback blocking, and user-flag downgrade attempts.

## Current policy stance

Personal Terminal is intentionally not a general shell. Unknown allowlisted commands are refused unless a concrete rule matches. Network, destructive, modifying, installer, shell, and arbitrary interpreter entrypoints remain blocked.

## Release gate status

This environment does not have Rust/Cargo or pnpm available, and Corepack could not download pnpm because registry access failed. I could not run:

```bash
cargo check --workspace
cargo test --workspace
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm test
```

`pnpm-lock.yaml` still needs to be generated in a pnpm-enabled dev environment with:

```bash
pnpm install
./scripts/release-check.sh
```

## Remaining note on `run_approved_task`

The execution path still accepts `repo_root` because the audit database is repo-scoped, so the backend needs the repo location to open the correct approval store. The actual argv/cwd/risk used for execution are still loaded from the approved command record, then repo-root-verified before execution.
