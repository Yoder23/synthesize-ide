# Synthesize v14.1 Revision Notes

V14.1 tightens the first code-execution surface introduced in v14. It does not add new IDE features.

## Key changes

- Governed task approval is now backend-snapshot based.
- `detect_tasks` persists backend-detected task snapshots.
- `approve_task` now accepts `task_id` only; it no longer accepts frontend-supplied `argv`, `cwd`, `requires_network`, or `may_modify_files`.
- `run_approved_task` runs only an approved persisted backend task snapshot.
- Arbitrary `node ...` execution is blocked by the command guard instead of being generally allowlisted.
- Package scripts are documented and labeled as repo-defined local code execution, not “safe” commands.
- Runtime/status wording now distinguishes:
  - agent-suggested commands: classification-only,
  - backend-detected tasks: executable after explicit approval through the governed task runner.
- V14 docs and README language were refreshed from stale v13 wording.

## Remaining production gate

Synthesize v14.1 still requires local release verification:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

Manual QA should include fake runtime, managed llama.cpp, manual local server, governed task detection/approval/run, and patch validate/approve/apply/rollback on a throwaway repo.

## Known limitations

- Governed tasks are not an unrestricted terminal.
- No OS-level network sandboxing.
- Package scripts execute repo-defined local code and require judgment even after approval.
- Lockfile generation and Rust/TypeScript build verification must be completed in a real dev environment.
