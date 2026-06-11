# Submission architecture

```mermaid
flowchart LR
  U[Developer] --> IDE[Synthesize IDE UI]
  IDE --> Ctx[Backend context builder]
  Ctx --> Audit[(Audit log)]
  Ctx --> LM[Local model runtime\nFake / llama.cpp / Ollama / LM Studio]
  LM --> Ops[Typed operations JSON]
  Ops --> Trace[Plan/action trace in chat]
  Ops --> Queue[Diff queue]
  Queue --> MoA[MoA / Synthesize governance layer]
  MoA --> Validate[Path + hash + policy validation]
  Validate --> Approve[Human approval]
  Approve --> Apply[Transactional apply + checkpoint]
  Apply --> Tests[Governed tasks / safe terminal]
  Tests --> Audit
  Apply --> Audit
  Validate --> Audit
  MoA --> Rollback[Rollback]
  Rollback --> Audit
```

## Trust boundary

The local model is outside the trusted action boundary. It can only emit typed operations. Synthesize/MoA owns validation, user approval, transactional apply, command policy, rollback, and audit persistence.

## Contest message

Synthesize IDE demonstrates how local/open agent tooling can make AI coding transparent and governable without requiring paid model services. Paid providers can be added as optional runtimes, but the safety architecture does not depend on them.
