# Context Visibility

Synthesize builds Assist context bundles and Studio/Dream Context Capsules in the Tauri backend before runtime generation.

The backend-owned context builder produces:

- persisted `context_bundle_id`
- exact runtime messages
- `messages_sha256`
- visible context preview
- included file/snippet metadata
- explicitly labeled conservative input-token estimate
- declared model window, output reservation, safety margin, and remaining capacity
- deterministic omissions and truncation records
- endpoint classification

`runtime_generate` no longer accepts raw frontend messages. It loads the persisted bundle by ID, verifies session and repo ownership, recomputes the messages hash, and sends only backend-derived messages.

File contents used for context must go through `RepoGuard`. Symlinked directories are skipped during file-tree traversal. Hidden and credential-like files are denied by default.

Studio uses the dedicated backend Context OS. It compiles a fresh role/task/spec-specific capsule, enforces a declared token budget before inference, redacts restricted records, and persists the exact capsule on every role invocation. Team exposes exact messages/hash and all budget and selection metadata without claiming to reveal private model chain-of-thought. See [Context Operating System](context-operating-system.md).
