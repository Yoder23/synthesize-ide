# Synthesize v10.1 Revision Notes

Synthesize v10.1 is a small security/polish hardening pass on top of v10. It does not add new model-runtime surfaces.

## Changes

- Hardened repo file-tree/context traversal so hidden and credential-like directories are not descended.
- Denied directory children are not exposed as model-context path names.
- Added a backend command to clear local session context/runtime/audit data while preserving patch lifecycle records and checkpoints.
- Added UI notice that exact context bundles/messages may include repo code and are stored locally in `.synthesize/synthesize-audit.sqlite`.
- Added Session Log action: **Clear local context/audit data**.
- Updated endpoint approval UX so non-local confirmation is set only after backend approval succeeds.
- Renamed the footer counter to runtime request attempts instead of completed endpoint calls.
- Strengthened the pre-apply warning to recommend clean Git branches/throwaway repos.
- Added tests/helpers for denied context directory policy.

## Known limitations

- Command execution remains disabled.
- No OS-level network sandbox.
- Runtime calls remain non-streaming.
- In-process repo mutation lock only.
- Custom diff applier supports only the documented text patch subset.
- No built-in model downloader/registry or llama.cpp process supervisor.
- `pnpm-lock.yaml` was not generated in this environment because pnpm is unavailable; generate and commit it in a real dev environment before enabling frozen lockfile CI.
