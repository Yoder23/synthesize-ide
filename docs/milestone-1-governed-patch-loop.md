# Milestone 1 — Governed Patch Loop

Synthesize's core milestone is the backend-authoritative patch loop:

```txt
model/fake runtime proposes typed operation
  ↓
frontend displays summary and diff
  ↓
backend validates and snapshots proposal
  ↓
backend records approval
  ↓
backend applies persisted snapshot with checkpoint/restore transaction shape
  ↓
backend owns rollback by proposal id
  ↓
audit/session log records lifecycle events
```

The frontend may request validation, approval, apply, and rollback. It is not the authority for any of them.

V8 extends this loop with an initial local OpenAI-compatible endpoint path while preserving the same invariant: endpoint output is untrusted JSON and can only propose typed operations.
