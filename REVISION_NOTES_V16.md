# Synthesize v16 Revision Notes

V16 is the first “daily editor” moonshot pass after the v15 open-source IDE foundation.

## What changed

- Added a multi-tab editor model with dirty indicators and close warnings.
- Added guarded Save Current File and Save All through backend `write_guarded_file`.
- Added guarded user file operations:
  - create file,
  - rename path,
  - delete file.
- Added Quick Open file picker.
- Added Command Palette for core workbench actions.
- Added persisted editor settings:
  - font size,
  - word wrap,
  - minimap,
  - light/dark Monaco theme.
- Added Git stage/unstage/commit UI.
- Added backend Git mutation commands:
  - `git_stage_file`,
  - `git_unstage_file`,
  - `git_commit_changes`.
- Git commit uses `--no-verify` to avoid executing repository hooks.
- Fixed a likely Rust compile blocker from duplicate derive on `RuntimeCancelResult`.

## Safety posture

V16 keeps the core Synthesize trust model:

- Model proposals do not directly write files.
- Agent patch apply still uses backend-owned proposal snapshots.
- Rollback still uses backend-bound checkpoint identity.
- User-initiated editor file operations go through RepoGuard.
- Governed task execution remains backend-detected, backend-approved, argv-only, timeout-bounded, and audited.
- Git operations are user-initiated backend commands and are audited.

## Honest limitations

V16 is not full VS Code parity. It still lacks full LSP JSON-RPC, debugger support, extension compatibility, remote dev, full terminal parity, and signed installers.

V16 remains a release candidate until the release gate passes and manual local-model QA is completed.
