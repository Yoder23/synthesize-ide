# Revision Notes v3

V3 narrows the project around Milestone 1: the governed patch loop.

## What changed

- Wired the desktop UI to the deterministic `FakeRuntimeAdapter`.
- Added a frontend path from fake generation to strict typed operation parsing.
- Added an operation timeline in the chat panel.
- Converted `propose_patch` operations into visible `DiffQueue` items.
- Added backend Tauri command `validate_patch_proposal`.
- Added backend Tauri command `apply_approved_patch`.
- Added backend Tauri command `read_guarded_file`.
- `open_repo_mock` now creates a real fixture repo in the system temp directory.
- The fake runtime now uses the real `beforeSha256` of the current editor content.
- Patch validation now flows through `RepoGuard`, patch shape validation, current commit validation, and `beforeSha256` checks.
- Patch apply now creates a `.synthesize/checkpoints/...` backup for touched files before writing.
- Patch apply records audit events into `.synthesize/synthesize-audit.sqlite`.
- The editor refreshes from the guarded backend read after patch application.
- `command-guard` now normalizes combined short flags such as `-fd`, `-df`, and `-fxd`.
- Added command guard tests for split/combined `git clean` flags and package-script approval classification.
- Added a minimal unified-diff applier in `patch-engine` for the fixture-grade patch path.

## What is intentionally still incomplete

- The diff applier is minimal and should be replaced or heavily hardened before general use.
- There is no real llama.cpp runtime launch yet.
- The command runner is still restricted subprocess mode, not a true OS sandbox.
- Git checkpointing is still file-backup based under `.synthesize/checkpoints`; it does not yet create a git stash/commit.
- Audit logging exists for the patch loop but is not yet surfaced in a full session replay UI.
- The repo opener is still a generated fixture repo, not a user-selected path.
- Network isolation is still a target, not an OS-enforced fact.

## Milestone 1 proof path

1. Open fixture repo.
2. Click `Build context + ask fake planner`.
3. Fake runtime emits a `propose_patch` operation.
4. Agent harness parses the operation with strict JSON/fenced JSON extraction.
5. Diff queue displays the proposed patch.
6. User validates through backend.
7. Backend validates via RepoGuard + patch engine.
8. User applies approved patch.
9. Backend checkpoints touched file, applies patch, records audit event.
10. Editor refreshes from guarded file read.

This is the first concrete proof that a model-proposed patch can travel through the governed path and safely land in a repo with validation, approval, checkpointing, and auditability.
