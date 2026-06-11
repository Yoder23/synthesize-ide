# Revision Notes V8

V8 makes Synthesize feel like a usable local coding-agent IDE while preserving the backend-authoritative patch lifecycle.

## What changed

- Added an initial OpenAI-compatible endpoint runtime adapter for already-running local model servers.
- Kept Fake Runtime available for deterministic governed patch-loop tests.
- Added runtime settings UI for provider, endpoint URL, model name, connection health, and remote endpoint confirmation.
- Added endpoint classification: localhost endpoints are treated as local; non-local endpoints show an explicit repo-context warning.
- Improved Agent Chat with message history, loading state, raw payload display, parse errors, and runtime display.
- Added a Synthesize system prompt that requires strict typed operations and prohibits claims of direct mutation.
- Built a practical context bundle for the agent: current file, hash, commit, file tree excerpt, dirty state, and constraints.
- Added context visibility panel with destination and approximate character estimate.
- Improved Diff Queue with changed-file summary, additions/deletions, risk notes, backend validation/approval/apply/rollback status, and visible Git/version-control warning before apply.
- Improved Session Log so audit events show human-readable proposal/approval/checkpoint summaries.
- Hardened the custom unified-diff applier for multi-hunk offsets, malformed hunk counts, context mismatches, binary patch rejection, rename/delete/mode-change rejection, and nested file creation tests.
- Made phase audit for applying/rolling_back best-effort so audit failure cannot strand a proposal after a lifecycle transition.
- Cleaned stale milestone-specific UI/backend strings.

## Scope intentionally not added

- No llama.cpp process supervisor.
- No GGUF model download/loading.
- No embeddings.
- No autocomplete.
- No command execution.
- No OS-level network sandbox.
- No major UI redesign.

## Trust state

Backend remains authoritative for validation, approval, apply, checkpoint binding, rollback, lifecycle transitions, and audit. The frontend does not provide raw patch contents during apply and does not provide checkpoint paths during rollback.

Command execution remains disabled; suggested commands are classification-only.
