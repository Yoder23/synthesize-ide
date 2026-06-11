# Synthesize VS Code Replacement Roadmap

Synthesize’s replacement thesis is focused: replace VS Code for workflows where local AI-governed coding is the primary loop, then absorb surrounding IDE workflows.

## Already differentiated

- Backend-owned context bundles.
- Exact context visibility.
- Self-hosted/local model runtime direction.
- Managed llama.cpp path.
- Typed model operation protocol.
- Backend-governed patch lifecycle.
- Backend approval and checkpointed apply.
- Backend-owned rollback.
- Audit/session log.
- Agent Profile policy enforcement.
- Governed task runner foundation.

## V16 editor foundation

- Multi-tab editor.
- Guarded save/refresh.
- Guarded file create/rename/delete.
- Quick Open.
- Command Palette.
- Editor settings persistence.
- Git stage/unstage/commit UI.

## Next replacement pillars

### V17: Real language intelligence

- TypeScript LSP client.
- Diagnostics panel.
- Hover/go-to-definition/find references.
- Document/workspace symbols.
- Format document.

### V18: Git and provenance

- Stage/unstage hunks.
- Working tree and staged diff views.
- Commit agent changes with provenance.
- Checkpoint/proposal history linked to commits.

### V19: Agent repair loop

- Feed governed task failures to the agent.
- Agent proposes repair patch.
- Validate/approve/apply.
- Repeat with max iterations and full audit.

### Later

- Debug Adapter Protocol.
- Plugin API.
- Remote/WSL/dev-container support.
- Packaging, signing, updates.
- Performance/indexing for large repos.

## Non-goals until safe

- Unrestricted shell execution.
- Cloud model presets.
- Full VS Code extension compatibility.
- OS-level sandbox claims before enforcement.
