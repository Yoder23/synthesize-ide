# Context Visibility

Synthesize v10 builds context bundles in the Tauri backend before runtime generation.

The backend-owned context builder produces:

- persisted `context_bundle_id`
- exact runtime messages
- `messages_sha256`
- visible context preview
- included file/snippet metadata
- character estimate
- endpoint classification

`runtime_generate` no longer accepts raw frontend messages. It loads the persisted bundle by ID, verifies session and repo ownership, recomputes the messages hash, and sends only backend-derived messages.

File contents used for context must go through `RepoGuard`. Symlinked directories are skipped during file-tree traversal. Hidden and credential-like files are denied by default.
