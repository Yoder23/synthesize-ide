# Release Checklist

Synthesize v19.2 should not be called personal-production ready until this checklist has been completed in a real development environment with Rust/Cargo and pnpm installed.

## Required automated gate

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

The gate must pass with a committed `pnpm-lock.yaml`.

## Required manual smoke tests

```text
Fake runtime patch loop passes.
Real local model patch loop returns a patch.
Context Sent to Model displays exact context.
Patch validates, approves, applies, and rolls back.
Governed Tasks can run a detected test/build command.
Personal Terminal allows pnpm test, cargo test, pytest, git status.
Personal Terminal blocks git add, git checkout, node script.js, pnpm exec, curl.
Terminal/task output can be sent to Agent Chat as a repair prompt.
Git diff displays the final applied changes.
```

## Required safety review

- Confirm model output cannot directly apply patches.
- Confirm frontend cannot apply raw patch content.
- Confirm rollback does not accept frontend checkpoint paths.
- Confirm Personal Terminal uses strict explicit-rule-only policy.
- Confirm user-entered command flags cannot downgrade command risk.
- Confirm dangerous Git commands are blocked in Personal Terminal.
- Confirm all command execution is argv-only, repo-bounded, timeout-bounded, env-scrubbed, output-bounded, and audited.

## Release label

Only after the automated gate and smoke tests pass should this build be labeled:

```text
Personal-production ready for local-model AI coding on personal repos and clean Git branches.
```
