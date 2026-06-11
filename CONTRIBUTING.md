# Contributing to Synthesize

Synthesize is a local-first AI-native IDE for self-hosted open-source coding models.

## Core invariants

- The model is untrusted.
- The frontend is not authority for approval/apply/run.
- Backend-owned context, runtime calls, patch lifecycle, task lifecycle, and audit are mandatory.
- Do not add cloud-provider defaults or API-key flows.
- Do not let model output directly mutate files or execute commands.

## Local checks

Before opening a PR:

```bash
pnpm install
./scripts/release-check.sh
```

This requires Rust/Cargo and pnpm.

## Areas that need help

- TypeScript LSP integration
- Git stage/commit/provenance UI
- Governed task runner hardening
- Diff engine hardening/replacement
- Local runtime adapters
- Manual QA with real local coding models
