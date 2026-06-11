# Synthesize v11.2 release-blocker fix

V11.2 is a narrow build-readiness patch on top of v11.1.

## Fixed

- Fixed the likely Rust `format!` compile blocker in `build_context_bundle` by adding the missing `agentProfile={}` placeholder to the backend-generated context prompt metadata.
- Preserved the v11.1 behavior where Agent Profile selection feeds into backend-owned context building and prompt generation.

## Not changed

This release does not add new product surface. It intentionally keeps the v11.1 scope:

- managed llama.cpp remains initial process supervision,
- command execution remains disabled,
- model downloads remain manual/import-only,
- local model requests remain backend-owned,
- patch validate/approve/apply/rollback remain backend-owned.

## Release checklist still required in a real dev environment

Run:

```bash
cargo check --workspace
cargo test --workspace
pnpm install
pnpm typecheck
pnpm build
pnpm test
```

Then commit `pnpm-lock.yaml` and switch CI back to `pnpm install --frozen-lockfile`.
