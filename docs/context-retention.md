# Context Retention

Synthesize persists exact context bundles locally so users can audit what was sent to a runtime. These bundles may include user prompts and repo code excerpts.

Location: `.synthesize/synthesize-audit.sqlite` inside the opened repo.

The Session Log includes **Clear local context/audit data**, which removes Studio Context Capsules, requests, summaries, runtime request rows, and audit events for the active session. It deletes unbound bundles and redacts agent-only compatibility bundle bodies while preserving IDs required by lifecycle records. Assist bundles still required by an active patch lifecycle remain readable so approval/apply policy is not silently broken. Patch and checkpoint records are preserved.

Current limitations:

- No retention schedule yet.
- No encrypted local store yet.
- No per-session export/import yet.
- Exact context persistence is local, but it can still contain sensitive code. Use a clean branch and clear session data when needed.

Studio role context is isolated by the Context OS. Each role receives only permitted record categories under a deterministic token budget; restricted business-context records are replaced with redacted metadata. The exact ordered capsule is persisted and bound to the role run and output operation.

Proof exports omit exact context and sensitive business context by default. Clearing a session removes context/runtime/audit material but intentionally does not silently destroy lifecycle, evidence, operation-hash, checkpoint, or rollback records required for accountability. There is still no encryption-at-rest or automatic retention schedule.
