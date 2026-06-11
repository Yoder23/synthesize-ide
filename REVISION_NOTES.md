# Revision Notes

This revision incorporates the first architectural review pass.

## Changed

- Footer trust claims now distinguish target state from verified enforcement.
- Command guard now uses structured argv rules instead of joined-string matching.
- `git push`, `git reset --hard`, `git clean -fd`, shell entrypoints, curl/wget, and package installs are explicitly covered.
- Added command-policy tests in the Rust crate.
- Added `proposalId`, per-file patch ids, current commit, and approval state types to patch proposals.
- Agent protocol Rust and TypeScript schemas were updated to match the stronger patch schema.
- JSON extraction no longer slices from first `{` to last `}`; it accepts strict JSON-only output or a fenced JSON block.
- Added a deterministic `FakeRuntimeAdapter` for developing the governed patch loop before real inference is wired.
- Audit schema now includes patch proposals, patch files, approval state, and external call records.
- Added `docs/milestone-1-governed-patch-loop.md`.

## Still not complete

- The patch engine validates proposal shape and hashes, but does not yet apply unified diffs.
- `sandbox-runner` is still restricted subprocess mode, not a full OS/container sandbox.
- Tauri commands are still mocked and need real backend wiring.
- Runtime supervision for llama.cpp is not implemented yet.
- External call monitoring is represented in schema/UI but not enforced.
