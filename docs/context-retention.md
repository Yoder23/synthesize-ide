# Context Retention

Synthesize persists exact context bundles locally so users can audit what was sent to a runtime. These bundles may include user prompts and repo code excerpts.

Location: `.synthesize/synthesize-audit.sqlite` inside the opened repo.

The Session Log includes **Clear local context/audit data**, which removes context bundles, runtime request rows, and audit events for the active session. Patch lifecycle records and checkpoint records are preserved so backend-owned rollback/accountability are not broken silently.

Current limitations:

- No retention schedule yet.
- No encrypted local store yet.
- No per-session export/import yet.
- Exact context persistence is local, but it can still contain sensitive code. Use a clean branch and clear session data when needed.
