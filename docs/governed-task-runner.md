# Governed Tasks and Personal Terminal

Synthesize v19.2 supports code execution for the personal AI coding loop through two bounded pathways:

1. **Governed Tasks** for backend-detected test/build/lint workflows.
2. **Personal Terminal** for a strict, explicit subset of user-entered safe iteration commands.

Neither pathway is a general shell. Both use argv-only process execution, canonical repo cwd checks, env scrubbing, timeouts, bounded output, reclassification, persisted approval, and audit records.

## Governed Tasks

Governed Tasks are discovered by the backend from repo metadata such as package scripts or common language conventions. The frontend can request detection, approval, and execution, but the backend owns the command snapshot.

```text
detect_tasks
  → backend persists command snapshot bound to canonical repo root
approve_task
  → frontend sends task_id only
  → backend loads persisted snapshot, verifies repo binding, reclassifies, and approves
run_approved_task
  → backend loads approved snapshot, verifies repo binding, reclassifies, and runs argv-only
```

Governed Tasks are intended for repeatable project tasks like tests, builds, and typechecks.

## Personal Terminal

Personal Terminal exists so the user can run the tight local repair loop without leaving Synthesize:

```text
run tests/build/lint
→ inspect output
→ send output to Agent Chat
→ receive repair patch
→ review/apply diff
→ run again
```

Personal Terminal uses `personal_terminal_policy()`, which is stricter than the detected-task policy:

- `require_explicit_rule = true`
- no allowlisted-program fallback
- unknown commands are blocked
- user flags cannot downgrade risk
- dangerous Git/network/mutation commands are blocked

## Allowed Personal Terminal commands

Allowed commands are intentionally narrow and local-iteration oriented:

```text
git status
git diff
git log
rg ...
ls
cat
pnpm test
pnpm run test|lint|build|typecheck
npm test
npm run test|lint|build|typecheck
yarn test
yarn run test|lint|build|typecheck
cargo test
go test ./...
pytest
dotnet test
```

## Blocked Personal Terminal examples

Commands outside explicit safe rules are blocked. Examples include:

```text
git add
git commit
git checkout
git switch
git restore
git merge
git rebase
git reset
git clean
git stash
git pull
git fetch
git push
npm install
pnpm install
yarn install
pnpm exec
node
python
bash
sh
curl
wget
rm
chmod
sudo
```

Git mutations belong in explicit Git UI flows, not Personal Terminal.

## Agent-suggested commands

Agent-suggested commands are classification-only. The model may suggest a command, but Synthesize does not execute model-proposed commands directly.

## Known limits

Synthesize does not claim OS-level sandboxing, network egress enforcement, container isolation, or full terminal emulation. Use Synthesize on personal repos and clean Git branches, and keep normal backups for valuable work.
