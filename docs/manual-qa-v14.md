# Manual QA v14

Run these checks on a clean Git branch or throwaway clone.

## Local agent loop

1. Start Synthesize.
2. Open fixture repo.
3. Use Fake Runtime.
4. Ask for a small patch.
5. Inspect exact context.
6. Validate, approve, apply, rollback.
7. Confirm Session Log events.

## Local model loop

1. Start managed llama.cpp or a manual local model server.
2. Health check through Synthesize.
3. Ask Local Patcher for a small change to the selected file.
4. Confirm typed JSON operation parses.
5. Validate, approve, apply, rollback.

## IDE foundation

1. Use Project Search for a symbol in the repo.
2. Open a result.
3. Refresh Git status.
4. Confirm changed files are listed after applying an agent patch.
5. Inspect Language Intelligence panel.

## Governed task runner

1. Detect tasks.
2. Approve a safe test task.
3. Run approved task.
4. Confirm stdout/stderr tails are bounded.
5. Confirm Session Log shows `task.approved`, `task.started`, and `task.finished` or `task.timed_out`.
6. Confirm network/destructive commands cannot be approved through the governed task path.

## Safety checks

1. Confirm there is no free terminal.
2. Confirm suggested agent commands are classification-only unless represented as detected tasks.
3. Confirm patch apply still uses backend persisted proposal snapshots.
4. Confirm rollback still uses proposal id, not frontend checkpoint path.
