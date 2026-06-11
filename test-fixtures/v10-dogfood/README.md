# v10 Dogfood Fixture Notes

Manual/backend test targets:

- Fake runtime should produce a `propose_patch` operation from a persisted context bundle.
- `runtime_generate` must use backend-loaded context bundle messages, not frontend-supplied prompt text.
- Non-local endpoints must require backend-persisted approval before repo context is sent.
- Symlinked directories should be skipped during file-tree traversal.
- `package.json` context reads must go through `RepoGuard`.
- Validate → approve → apply → rollback should remain backend-owned and proposal-id keyed.
