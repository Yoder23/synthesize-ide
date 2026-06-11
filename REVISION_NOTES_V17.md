# Synthesize v17 — VS Code-Replacement Hardening + Workbench Polish

V17 continues the moonshot direction while keeping the trust model intact.

## What changed

- Hardened guarded delete operations:
  - rejects empty path, `.`, repo root, `.git`, and `.synthesize` paths,
  - requires an explicit confirmation token for directory deletes,
  - audits rejected delete attempts where possible.
- Added read-only Git diff preview:
  - unstaged diff per file,
  - staged diff per file,
  - backend command is read-only and RepoGuard-checked,
  - diff output is bounded/truncated.
- Added a lightweight Problems panel:
  - TODO/FIXME markers,
  - not-implemented placeholders,
  - simple brace-balance checks,
  - clearly labeled as lightweight local checks, not full LSP diagnostics.
- Kept governed tasks backend-owned:
  - backend-detected task snapshots,
  - backend approval,
  - reclassification before run,
  - argv-only execution,
  - env scrub, timeout, bounded output, audit.
- Preserved backend-governed patch lifecycle and self-hosted local model direction.

## What v17 is not

V17 is not full VS Code parity. It still lacks full LSP JSON-RPC, DAP debugging, extensions, remote/dev-container support, packaged installers, and signed releases.

## Required release gate

Run locally before any production claim:

```bash
pnpm install
git add pnpm-lock.yaml
./scripts/release-check.sh
```

Then manually QA fake runtime, managed llama.cpp, local model server, governed patch apply/rollback, Git diff/stage/commit, and governed tasks.
