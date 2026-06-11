# Synthesize v9 Revision Notes

V9 makes Synthesize dogfood-ready as a local-first coding-agent IDE with backend-governed patch application.

## Main changes

- Moved OpenAI-compatible endpoint calls from the frontend into Tauri backend commands.
- Added backend endpoint classification: `local`, `private-lan`, and `remote`.
- Added backend-owned endpoint approval for non-local endpoints before repo context is sent.
- Added persisted context bundles so the Context Visibility panel can show the exact messages sent to the runtime.
- Added backend runtime audit events for context creation, health checks, generation start/completion/failure, and endpoint approval.
- Kept Fake Runtime deterministic and available through the backend runtime path.
- Updated Agent Chat to use backend `build_context_bundle` and `runtime_generate`.
- Updated Runtime Control to test endpoint health and list models through the backend.
- Keyed validation, approval, apply, and rollback UI state by `proposal_id`.
- Improved Diff Queue changed-file summary and command suggestions as classification-only.
- Added tests for endpoint classification, endpoint approval enforcement, and fake runtime typed operation output.
- Added CI workflow and check scripts.

## Still intentionally not implemented

- Command execution.
- Built-in model downloads.
- llama.cpp/GGUF process supervision.
- Embeddings.
- Autocomplete.
- Multi-agent orchestration.
- OS-level network sandboxing.

## Trust state

- Patch approval is backend-owned.
- Patch apply consumes persisted proposal snapshots only.
- Rollback consumes backend-bound checkpoint identity only.
- OpenAI-compatible endpoint calls go through the backend.
- Non-local endpoint use requires backend-persisted user approval before repo context is sent.
- Command execution remains disabled.
- Network sandboxing is not OS-enforced.
