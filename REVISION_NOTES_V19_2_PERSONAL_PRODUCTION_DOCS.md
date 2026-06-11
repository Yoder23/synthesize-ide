# Synthesize v19.2 — Personal-production docs and release gate alignment

This revision intentionally avoids feature expansion. It catches the documentation and release checklist up to the v19/v19.1 personal agent loop and strict Personal Terminal policy.

## What changed

- Updated `README.md` from stale v16/v18 language to v19.2.
- Documented the three command pathways:
  - Governed Tasks
  - Personal Terminal
  - Agent-suggested command classification
- Documented strict Personal Terminal command policy and examples.
- Updated `docs/governed-task-runner.md` to include Personal Terminal.
- Updated `docs/production-readiness.md` with v19.2 release criteria.
- Updated `docs/known-limitations.md` to reflect strict user-entered command execution.
- Updated `RELEASE_CHECKLIST.md` with command-policy smoke tests.
- Updated `SECURITY.md` with Personal Terminal policy-bypass reporting scope.
- Added `docs/v19.2-personal-production.md`.

## Release gate status

This environment has Node/Corepack but could not download pnpm from the registry, and Rust/Cargo are unavailable. I could not generate `pnpm-lock.yaml` or run `./scripts/release-check.sh` here.

The required local steps remain:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

## Production-readiness stance

Synthesize v19.2 is a personal-production ready candidate. After the lockfile is committed, the release gate passes, and the real local-model smoke test passes, it is reasonable to call it personal-production ready for local-model AI coding on personal repos and clean Git branches.
