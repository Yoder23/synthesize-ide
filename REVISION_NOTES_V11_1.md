# Synthesize v11.1 release-candidate hardening

V11.1 is a targeted release-candidate hardening pass after the first local-model ownership release.

## Changed

- Managed llama.cpp stdout/stderr are now consumed by bounded background readers so the child process cannot block on full pipe buffers.
- Managed llama.cpp status now calls `try_wait()` and reports exited processes as failed/stopped instead of continuing to show a stale tracked process.
- Managed llama.cpp status returns bounded stdout/stderr tails for diagnostics.
- Local model/runtime metadata storage moved from a temporary directory to an app-data style directory:
  - `SYNTHESIZE_APP_DATA_DIR` override when set,
  - `%APPDATA%/SynthesizeIDE` on Windows,
  - `~/Library/Application Support/SynthesizeIDE` on macOS,
  - `$XDG_DATA_HOME/synthesize-ide` or `~/.local/share/synthesize-ide` on Linux,
  - temp only as final fallback.
- Agent Profile selection is now wired into the backend context builder and system prompt.
- Local Planner, Local Patcher, Local Reviewer, and Fake Demo Agent produce different prompt guidance.
- Runtime UI displays managed llama.cpp log tails when available.
- Documentation now distinguishes v11.1 release-candidate status from v11 dogfood status.

## Still not changed

- Synthesize still does not download models automatically.
- Command execution remains disabled.
- llama.cpp must still be installed/built separately.
- CI/build verification still needs a real Rust + pnpm environment.
- No `pnpm-lock.yaml` was generated in this environment because pnpm is unavailable.

## Recommended verification

Run in a real dev environment:

```bash
cargo check --workspace
cargo test --workspace
pnpm install
pnpm typecheck
pnpm build
pnpm test
```

Then generate and commit `pnpm-lock.yaml` with:

```bash
pnpm install
```

After the lockfile is committed, switch CI back to `pnpm install --frozen-lockfile`.
