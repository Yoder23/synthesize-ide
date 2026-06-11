# Production-Shaped MVP Spec

## Non-negotiable requirements

- Local-only mode indicator.
- Runtime adapter contract.
- Model profile separate from agent profile.
- Structured operation protocol.
- Context visibility before generation.
- Diff-first patch proposal and review.
- Checkpoint before apply.
- Command request approval.
- No raw shell by default.
- Audit log for session replay.
- External call counter.

## Definition of done for first vertical slice

- Open local repo.
- Import GGUF model.
- Launch llama.cpp runtime.
- Chat with selected-file context.
- Show context bundle.
- Log model request/response.

## Definition of done for full MVP

- Multi-agent plan/patch/review/test loop.
- Hunk-level patch acceptance.
- Guarded command runner with risk panel.
- Privacy report: external calls, runtime, model, context, commands, patches.
- Rollback from checkpoints.

