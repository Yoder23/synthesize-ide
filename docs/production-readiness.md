# Production Readiness

Synthesize v19.2 is a personal-production ready candidate for a local-first AI IDE. The architecture is production-shaped for personal use, but the build must pass the release gate and real local-model smoke tests before it should be called production-ready for daily dogfooding.

## Ready candidate areas

- Backend-owned context bundles and model calls.
- Typed model operation protocol.
- Backend-governed patch validation, approval, apply, rollback, lifecycle transitions, and audit logging.
- Exact context visibility.
- Local model server and managed llama.cpp workflows.
- Multi-tab editor, inline selection prompts, quick open, command palette, search, problems, and basic Git UI.
- Governed task runner for backend-detected tests/builds/lints.
- Strict Personal Terminal for safe user-entered local iteration commands.
- Terminal/task output handoff back to Agent Chat for repair prompts.

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
Context Sent to Model shows exact bundled context.
Patch validates, approves, applies, and rolls back.
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

## Personal-production verdict

Once the lockfile is committed, `./scripts/release-check.sh` passes, and the manual local-model smoke tests pass, Synthesize v19.2 can be considered personal-production ready for local-model AI coding on your own repos and clean Git branches.
