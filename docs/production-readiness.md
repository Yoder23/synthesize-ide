# Production Readiness

Synthesize v19.3 is a personal-production ready candidate for a local-first AI IDE with optional cloud heavy-lift lanes. The architecture is production-shaped for personal use, but the build must pass the release gate and real runtime smoke tests before it should be called production-ready for daily dogfooding.

## Ready candidate areas

- Backend-owned context bundles and model calls.
- Backend-owned Context OS with registered runtime capabilities, fresh bounded role capsules, exact-message hashes, deterministic pruning/retrieval, and pre-inference overflow rejection.
- Typed model operation protocol.
- Backend-governed patch validation, approval, apply, rollback, lifecycle transitions, and audit logging.
- Exact context visibility.
- Local model server and managed llama.cpp workflows.
- Optional cloud-heavy provider lanes (OpenAI/Anthropic) with endpoint approval and explicit key requirements.
- Multi-tab editor, inline selection prompts, quick open, command palette, search, problems, and basic Git UI.
- Governed task runner for backend-detected tests/builds/lints.
- Strict Personal Terminal for safe user-entered local iteration commands.
- Terminal/task output handoff back to Agent Chat for repair prompts.
- Skill-agent orchestration with configurable hand-off graph and sequential queue execution.
- Outcome-Governed Studio with immutable specifications, nine role contracts, backend state machines, evidence gates, and privacy-safe proof reports.
- Mandate-governed Dream inbox and explicit human promotion/autonomy controls.
- Real Git worktree isolation with clean-tree, stale-base, identity, and cleanup checks.
- Symbolic Pulse plus deterministic elapsed-time rule monitoring; the liquid observer remains experimental and shadow-only.

## Must pass before personal-production use

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

The release gate requires Rust/Cargo and pnpm. It verifies:

```text
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm test
```

## Manual smoke tests

Run these after the automated release gate:

```text
Fake runtime patch loop passes.
Real local model patch loop returns a patch.
Cloud heavy-lift request succeeds with approved endpoint and valid API key.
Context Sent to Model shows exact bundled context.
Studio Team shows each role capsule's full token equation, inclusions, omissions, summaries, truncations, exact messages, and hash.
Changing a role's runtime/model capability forces a newly compiled budget; stale spec/ADR capsules are rejected.
Patch validates, approves, applies, and rolls back.
Skill queue runs one local Qwen3 skill at a time, and hand-off constraints are enforced.
Studio success/revision/replan/blocking/budget/malformed-output scenarios route correctly.
Dream deduplication, human autonomy elevation, promotion, and no-active-branch-write policy hold.
Governed worktree creation/diff/cleanup leaves the active checkout unchanged.
Proof export contains operation/evidence provenance and excludes exact context.
Pulse labels the liquid observer experimental and gives it no authority.
Personal Terminal allows pnpm test, cargo test, pytest, git status.
Personal Terminal blocks git add, git checkout, node script.js, pnpm exec, curl.
Terminal output can be sent back to Agent Chat as a repair prompt.
Git diff reflects applied patches.
```

## Not claimed

Synthesize v19.2 is not:

- full VS Code parity
- an extension marketplace
- a full debugger/DAP implementation
- a full LSP implementation
- an enterprise sandbox
- an OS-level network isolation system
- a safe environment for untrusted repos
- a replacement for clean Git branches and backups
- proof that an outcome-pending business result has occurred
- autonomous Dream merge authority

## Personal-production verdict

Once the lockfile is committed, `./scripts/release-check.sh` passes, and the manual local/cloud smoke tests pass, Synthesize v19.3 can be considered personal-production ready for AI coding on your own repos and clean Git branches.
