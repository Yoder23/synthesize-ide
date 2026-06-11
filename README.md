# Synthesize IDE

## Synthesize v19.3 status

Synthesize v19.3 is a personal-production ready candidate for a local-first AI coding workbench. It is designed for developers who want to use self-hosted open-source coding models to edit local repos through a governed workflow instead of a cloud AI coding service. This candidate adds a **MoA Action Planner** profile: the local model can plan and propose actions through chat, while Synthesize/MoA governance remains responsible for validation, approval, application, rollback, and audit.

The core invariant remains:

> The model never acts. The model proposes typed operations. The trusted backend validates, records approval, applies changes transactionally, and audits everything.

v19.3 does not add broad IDE scope. It adds the contest/demo-oriented MoA Action Mode, documents the submission workflow, and keeps the release gate explicit.

## What works in v19.3

- Local/self-hosted model workflow with Fake Runtime, Local Model Server, and Managed llama.cpp.
- Backend-owned context bundles and backend-owned model calls.
- Exact context visibility.
- Typed operation parser.
- Backend-governed patch validation, approval, checkpointed apply, and rollback.
- Multi-tab Monaco editor with dirty indicators, guarded save/save-all/refresh, quick open, command palette, and settings.
- Inline selection actions: explain selection, fix selection, write tests.
- Project Search and lightweight Problems panel.
- Git status, diff preview, stage/unstage, and commit using audited backend commands.
- Governed task runner for backend-detected tests/builds.
- Personal Terminal for a strict safe subset of user-entered local iteration commands.
- Task and terminal output can be fed back to Agent Chat for a repair loop.
- Session/audit log.
- **MoA Action Planner** profile for local-model planning with typed operations and a visible plan/action trace.

Synthesize v19.3 is not full VS Code parity and not enterprise production security. It is intended for personal daily dogfooding on local repos and clean Git branches after the release gate and smoke tests pass.

OpenAI-compatible means local HTTP wire protocol only. Synthesize does not require an OpenAI account or OpenAI API key.

## Agent Competition submission

For Microsoft Agent competition packaging and judging prep, start here:

- `COMPETITION_SUBMISSION.md` for the finalized competition packet.
- `SUBMISSION_READY.md` for the practical local release/demo checklist.
- `docs/submission-architecture.md` for the trust-boundary architecture diagram.
- `docs/moa-bridge.md` for the bundled MoA bridge contract.

Quick prep commands (Windows PowerShell):

```powershell
./scripts/submission-check.ps1
./scripts/submission-bundle.ps1
```

## Submission pitch

Synthesize IDE lets local AI models plan and propose code changes, but only a trusted backend can validate, approve, apply, roll back, and audit those actions. It makes agentic coding observable, interruptible, reversible, and safe-by-design.

Recommended contest track: **Creative Apps**.

## Personal-production workflow

```text
Open repo
→ start/connect local model
→ open file
→ select code or ask chat
→ agent proposes patch
→ review diff
→ validate/approve/apply
→ run tests/builds/lints in Governed Tasks or Personal Terminal
→ feed failures back to agent
→ apply repair patch
→ inspect Git diff
→ commit through the explicit Git workflow
```

## Run Synthesize

```bash
pnpm install
pnpm desktop:tauri
```

For frontend-only development:

```bash
pnpm desktop:dev
```

## Use MoA Action Mode

1. Open a repo.
2. Select **MoA Action Planner** in Local Agent Profile.
3. Use **Draft MoA action** or enter a coding goal in Agent Chat.
4. The local model creates a plan/action trace and emits Synthesize typed operations.
5. Review the diff queue.
6. Validate, approve, apply, run tests, and inspect the audit log.

The chat window shows a plan/action trace derived from typed operations. It does not claim to expose private model chain-of-thought. See `docs/moa-action-mode.md`.

## Use Fake Runtime end-to-end

1. Start Synthesize.
2. Open the fixture repo.
3. Select **Fake runtime**.
4. Ask for a small change.
5. Inspect exact context in **Context Sent to Model**.
6. Review the patch in **Diff Queue**.
7. Validate + snapshot in backend.
8. Approve persisted proposal.
9. Apply approved snapshot.
10. Roll back if needed.
11. Inspect Session Log.

## Use a local model server

1. Start a self-hosted local model server, such as llama.cpp server, LM Studio, Ollama local route, or vLLM.
2. Select **Local model server** in Synthesize.
3. Pick a preset or enter a localhost URL, for example:

```txt
http://localhost:8080/v1
```

4. Enter the model name expected by your server.
5. Run the backend health check.
6. Ask the Local Agent for a change.

Private-LAN and remote endpoints require explicit backend approval before repo context is sent.

## Use managed llama.cpp

Synthesize can start a user-provided llama.cpp server binary with a user-provided GGUF model file.

1. Download/build llama.cpp separately.
2. Download a GGUF open-source coding model separately.
3. In Synthesize, import the local GGUF path in Model Library.
4. Enter the llama.cpp server binary path.
5. Enter port/context size.
6. Start managed llama.cpp.
7. Health check the generated localhost endpoint.
8. Use the normal agent/diff/apply/rollback workflow.

Synthesize starts the process with argv-only process APIs and binds to `127.0.0.1` by default. This is not a full sandbox.

## Code execution model

Synthesize has three command pathways with different trust rules.

### Governed Tasks

Backend-detected task snapshots for common repo workflows:

```text
detect_tasks
  → backend persists task snapshot bound to canonical repo root
approve_task
  → frontend sends task_id only
  → backend loads persisted snapshot, verifies repo binding, reclassifies, and approves
run_approved_task
  → backend loads approved command snapshot, verifies repo binding, reclassifies, and runs argv-only
```

Governed Tasks are for detected test/build/lint workflows. They are persisted, approved, bounded, env-scrubbed, and audited.

### Personal Terminal

Personal Terminal is for safe local iteration commands entered by the user. It uses a stricter policy than detected tasks:

- Explicit allow rules only.
- No allowlisted-program fallback.
- User-provided network/file-modification hints are recorded but cannot downgrade risk.
- Unknown commands are blocked.
- Mutating/network Git commands are blocked.

Allowed examples:

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

Blocked examples:

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

### Agent-suggested commands

Agent-suggested commands are classification-only. The model may suggest a command, but Synthesize does not execute model-proposed commands directly.

## Safety model

- Model output is untrusted.
- The frontend is not authority for approval or apply.
- The backend owns context bundles and runtime calls.
- The backend owns patch proposal snapshots, operation hashes, approvals, checkpoint identities, apply/rollback, lifecycle transitions, and audit records.
- Apply does not accept raw frontend patch content.
- Rollback does not accept frontend checkpoint paths.
- Task and Personal Terminal execution use argv-only spawn, cwd guard, env scrub, timeout, bounded output, reclassification, and audit.
- Personal Terminal uses strict explicit-rule-only command policy.
- Agent-suggested commands remain classification-only.
- No OS-level network sandbox is claimed.

## Production release gate

Run the local release gate before tagging a production build:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

This requires Rust/Cargo and pnpm. The release gate runs Rust checks/tests and frontend typecheck/build/test.

## v19.2 smoke tests

After the release gate passes, do one real local-model smoke test and one command-policy smoke test:

```text
Fake runtime patch loop passes.
Real local model patch loop returns a patch and context is visible.
Patch validates, approves, applies, and rolls back.
Personal Terminal allows: pnpm test, cargo test, pytest, git status.
Personal Terminal blocks: git add, git checkout, node script.js, pnpm exec, curl.
Terminal output can be sent back to Agent Chat as a repair prompt.
```

## Recommended docs

- `docs/local-model-runtime.md`
- `docs/llama-cpp-setup.md`
- `docs/model-library.md`
- `docs/runtime-presets.md`
- `docs/agent-profiles.md`
- `docs/context-visibility.md`
- `docs/governed-task-runner.md`
- `docs/v19.2-personal-production.md`
- `docs/production-readiness.md`
- `docs/known-limitations.md`
- `CONTRIBUTING.md`
- `SECURITY.md`

## Honest release status

Synthesize v19.2 is a personal-production ready candidate until the local release gate and manual local-model smoke tests pass. Do not use it on high-value production/client repos without clean Git branches and backups.
