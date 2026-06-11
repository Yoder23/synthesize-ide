# Synthesize v17 VS Code-Replacement Progress

Synthesize's replacement strategy is not to clone VS Code horizontally. The goal is to replace VS Code for workflows where local, governed AI coding is the primary loop.

## Strengthened in v17

- Safer backend file deletion.
- Git diff preview beside stage/unstage/commit.
- Lightweight Problems panel until real LSP diagnostics land.
- Continued backend-governed patch lifecycle.
- Continued backend-governed task runner.

## Current replacement bar

Synthesize is closest to replacing VS Code when the daily workflow is:

1. Open a local repo.
2. Edit files with tabs/settings/quick open.
3. Search and inspect Git status/diffs.
4. Ask a self-hosted local model for a patch.
5. Validate, approve, apply, and rollback through the backend.
6. Run governed tests/build tasks.
7. Commit locally.

## Biggest remaining gaps

- Real LSP JSON-RPC client.
- TypeScript diagnostics/hover/go-to-definition/find references.
- Debug Adapter Protocol.
- Extension/plugin system.
- Signed installers and updates.
- Full Git conflict/history/stash workflows.
- Streaming model responses.

## V18 recommendation

Make TypeScript LSP real: diagnostics panel, inline diagnostics, hover, go-to-definition, find references, document symbols, and format document.
