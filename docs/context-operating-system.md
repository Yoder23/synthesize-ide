# Context Operating System

Synthesize treats a model window as temporary working memory. The durable source of truth is the repository-local intent/evidence ledger. Before every Studio or Dream role invocation, the Rust backend compiles a new bounded, role-specific Context Capsule; role transcripts are never concatenated into a shared prompt.

## Runtime capability contract

Every configured runtime/model pair has a persisted capability record: context-window tokens, maximum output tokens, safety margin, token-count method, structured-output behavior, provenance, and validation time. An invocation is rejected before network or local-model inference unless:

```text
compiled input tokens + reserved output tokens + safety margin
<= declared model context window
```

`runtime_tokenizer` may be selected only when a matching tokenizer is actually available. The current general adapter uses the explicitly labeled conservative UTF-8-byte estimate (`ceil(bytes / 3)`). Legacy character counts retain the label `legacy_character_count_not_tokens`; they are never represented as token counts.

## Capsule compilation

`context-os` loads versioned ledger records, applies one of nine explicit role projection policies, checks mandatory inputs, and orders material by `P0_PROTOCOL` through `P4_BACKGROUND`. P0 and mandatory P1 records are indivisible. Optional records are pruned deterministically by role/category allocation and priority. Material is classified as:

- hot: included in the exact messages;
- warm: omitted from this invocation but available through a validated request or structured summary;
- cold: retained in authoritative history and excluded by default.

Each persisted capsule contains identity and task bindings, active spec and ADR versions, runtime capability, exact/estimated counts, included/omitted/summarized records, truncations, source hashes, exact messages, message hash, and creation time. Before generation and again before accepting an operation, the backend rejects stale spec or ADR bindings.

Repeated runs for the same role and task use source hashes to omit unchanged optional material. Versioned summaries cover initiative, role, task, repository, evidence, and completed-phase views. They disclose omissions, remain non-authoritative, and become stale when a reloaded source hash changes.

## Progressive disclosure and repository safety

The typed `request_context` operation supports requirements, ADRs, UX criteria, assumptions, evidence, findings, task summaries, repository maps, file excerpts, symbols, definitions, references, direct dependencies, and tests. The backend validates role permissions and selectors, applies RepoGuard to canonical repository-relative paths, recompiles the budget, persists the resulting capsule, records the request, and continues the same serialized role run with the new exact messages. A run is limited to four progressive requests before it must partition or narrow its task. Models cannot use this operation as an arbitrary file-read primitive.

Repository retrieval is deterministic and staged: bounded map, lexical symbol/signature and direct-dependency signals, target implementation, tests, then wider references when requested. The interface is intentionally independent of embeddings so a future hybrid retriever will not require a protocol change.

## Failure and drift behavior

Missing mandatory records and mandatory overflow block inference. A bound task transitions to `BLOCKED_CONTEXT_OVERFLOW`; the event explains that the operator may partition the task, narrow the spec, or refresh context. Pulse deterministically observes stale bindings, missing/overflow events, repeated requests, summary conflicts/staleness, capsule churn, low-priority dominance, failures after omission, and token-pressure trends. The experimental liquid observer may observe these facts but cannot determine context truth or override a block.

## Visibility and retention

Assist and Studio both display the exact backend-owned messages and hash plus the declared window, output reservation, margin, input tokens, remaining capacity, method, inclusion/omission decisions, summaries, and truncations. Assist's selected-file metadata reports the actual excerpt length, not the source file length.

Exact context can contain proprietary code. **Clear local context/audit data** deletes Studio capsules, context requests, summaries, runtime requests, and audit events for the active session. It redacts agent-only compatibility bundle bodies while retaining required lifecycle IDs; Assist bundles still bound to an unapplied patch remain until that lifecycle no longer requires them. No encryption-at-rest or automatic retention schedule is claimed.

## Recovery

Capsules and runtime capabilities are SQLite records and restore after restart. A restored capsule is useful for audit, but it is not reused for a new invocation: the backend always compiles a fresh capsule and repeats capability, sufficiency, token-budget, and freshness checks.
