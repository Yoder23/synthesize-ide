# Synthesize v18 Personal-Production Workflow

Synthesize v18 targets the personal local-model workflow rather than public VS Code parity.

The workflow to validate is:

```text
Open repo
→ start or connect local model
→ open file
→ select code or ask chat
→ agent proposes typed patch
→ review diff
→ validate/approve/apply
→ run governed test/build task
→ feed task output to agent if needed
→ agent proposes repair patch
→ apply/rollback
→ inspect Git diff
→ commit
```

## Inline help

The editor toolbar includes:

- Explain selection
- Fix selection
- Write tests

These actions only draft prompts. They do not give the model direct file access. Synthesize still builds the backend-owned context bundle when the user asks the agent.

## Task-output repair loop

The Governed Tasks panel can send bounded stdout/stderr output back to Agent Chat. This is the first practical repair loop:

```text
run approved task
→ task fails
→ feed output to agent
→ model proposes repair patch
→ validate/approve/apply
```

Task output is treated as untrusted context. It may influence model proposals, but only backend-governed patch operations can modify files.

## Code execution boundary

Synthesize v18 does not include a free terminal. Code execution is limited to backend-detected tasks that are:

- persisted as task snapshots,
- approved by task id,
- reclassified before run,
- executed argv-only,
- cwd-guarded through RepoGuard,
- env-scrubbed,
- timeout-bounded,
- audited.

## Current limitations

- Build gate still must pass locally.
- No real LSP JSON-RPC client yet.
- Task cancellation is not fully async yet; timeout remains the hard backstop.
- No debugger/DAP.
- No extension/plugin system.
- No signed installers or auto-update.
