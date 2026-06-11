# Synthesize v12.1 release-candidate cleanup

This release is intentionally narrow. It does not add product features.

## Changes

- Replaced stale `Synthesize v11` wording in current docs with milestone-neutral language.
- Updated GitHub Actions frontend install step to require a committed `pnpm-lock.yaml` and use `pnpm install --frozen-lockfile`.
- Added `RELEASE_CHECKLIST.md` with the exact lockfile, build, test, and local-model smoke-test gate required before production claims.

## Still required

- Generate and commit `pnpm-lock.yaml` in a real pnpm-enabled environment.
- Run `./scripts/release-check.sh` in an environment with Rust/Cargo and pnpm.
- Smoke-test fake runtime, managed llama.cpp, manual llama.cpp server, and a real local coding-model patch on a throwaway repo.

## Status

Synthesize v12.1 remains a release candidate until the release gate and manual QA pass.
