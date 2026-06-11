# Synthesize v11 Revision Notes

V11 reframes Synthesize around self-hosted open-source coding models rather than “OpenAI-compatible endpoint” product language. OpenAI-compatible remains an implementation protocol for local model servers only.

## Added

- Local Model Runtime Control Center language and UX.
- Runtime presets for llama.cpp server, LM Studio local server, Ollama local route, vLLM local server, and custom local servers.
- First-pass managed llama.cpp path:
  - import an existing llama.cpp server binary path,
  - import/select a local GGUF model path,
  - start the server with argv-only process spawning,
  - bind to `127.0.0.1` by default,
  - stop/status commands.
- Model Library metadata path for importing local `.gguf` files.
- Local Agent Profile panel with Local Patcher/Planner/Reviewer/Fake Demo concepts.
- Stronger local coding agent prompt language.
- Local-first docs for GGUF, llama.cpp, LM Studio, Ollama, vLLM, runtime presets, agent profiles, and manual QA.

## Preserved invariants

- Backend owns context bundles and runtime generation.
- Frontend does not send raw prompts/messages to the model runtime after context bundle creation.
- Model output remains untrusted typed operations.
- Patch apply uses backend-persisted proposal snapshots.
- Rollback uses backend-bound checkpoint identity.
- Command execution remains disabled.

## Known limitations

- Managed llama.cpp expects the user to provide an already-built llama.cpp server binary and a local GGUF model.
- No automatic model downloads are implemented in v11.
- Runtime calls are non-streaming.
- Managed process supervision is initial and in-process; it is not a full production supervisor.
- No OS-level network sandbox is implemented.
- The diff applier supports only the documented text unified-diff subset.
