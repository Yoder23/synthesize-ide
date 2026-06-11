# Revision notes: V4

V4 focuses on making the Milestone 1 governed patch loop harder to fool and closer to a usable local-first IDE workbench.

## Major changes

- Added real local repo path opening alongside the fixture repo.
- Added guarded repo file navigation and backend file reads.
- Added dirty-buffer detection; planning is disabled when Monaco content no longer matches disk.
- Added Session Log panel backed by `list_audit_events`.
- Added rollback from the last checkpoint.
- Added backend command classification path wired to CommandApproval.
- Added runtime status, curated model lanes, and local GGUF path registration scaffolding.
- Hardened patch validation:
  - unified diff must include `diff --git`/`---`/`+++` markers,
  - unified diff must include at least one `@@` hunk,
  - diff header paths must match `PatchFile.path`,
  - patch application rejects no-op results.
- Added rejected patch fixtures for no-hunk, path mismatch, denied env, and outside-repo cases.

## Still incomplete and not overclaimed

- Cargo/Rust checks were not run in this environment.
- The real llama.cpp supervisor is scaffolded but not complete.
- Model downloads are not yet checksum-verified from the UI.
- Command execution is not enabled from the UI.
- OS-level command/network sandboxing is not complete.

## Core invariant preserved

The model never acts. The model proposes typed operations. The harness validates, displays, and executes only approved operations.
