# Synthesize v15 Revision Notes

V15 is the open-source IDE foundation release candidate.

## Main changes

- Fixed the task snapshot repo-scope data model by changing `task_snapshots` to use `PRIMARY KEY(task_id, session_id, repo_root)`.
- Updated `approve_task` to query snapshots by `task_id + session_id + canonical repo_root`.
- Added migration logic for older task snapshot tables.
- Added tests for reusing the same `task_id` across multiple repos in one session.
- Preserved backend-owned task approval/run flow.
- Updated current docs from stale v13/v14 wording to v15.
- Added open-source project docs: `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`.
- Added `docs/v15-open-source-ide.md`.

## Release status

V15 is still a release candidate until:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

passes in a real Rust/pnpm environment, followed by manual fake-runtime, managed llama.cpp, local model server, patch lifecycle, and governed-task smoke tests.
