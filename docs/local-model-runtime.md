# Local Model Runtime

Synthesize is designed for self-hosted open-source coding models. It does not require an OpenAI account or OpenAI API key.

Runtime modes:

- **Fake runtime**: deterministic fixture model for testing the governed patch loop.
- **Local model server**: a user-run local server using the OpenAI-compatible local HTTP protocol.
- **Managed llama.cpp**: Synthesize starts a local llama.cpp server binary with a local GGUF model path. The scripts can bootstrap both without Ollama.

“OpenAI-compatible” in Synthesize means protocol shape only. It is a common local HTTP API implemented by llama.cpp server, LM Studio, vLLM, and some Ollama routes. It does not mean Synthesize uses OpenAI cloud.

Non-local model servers require explicit backend approval before Synthesize sends repo context.


## No-Ollama bootstrap

```powershell
./scripts/bootstrap-local-model.ps1 -Model smoke
./scripts/start-local-model.ps1
./scripts/local-model-smoke.ps1
```

This pulls a Qwen2.5 Coder GGUF model into `.synthesize-runtime/models`, downloads a local llama.cpp server binary into `.synthesize-runtime/llamacpp`, starts the server on `127.0.0.1`, and checks that the real model emits Synthesize typed operations accepted by the MoA bridge.

## v11.1 release-candidate notes

- Managed llama.cpp status now detects exited child processes.
- Managed llama.cpp stdout/stderr are consumed with bounded log tails to avoid pipe-buffer blocking.
- Local model/runtime metadata now uses an app-data directory instead of a temporary directory when possible.
- Agent Profile selection now affects the backend-generated local agent system prompt.
- Generate and commit `pnpm-lock.yaml` in a real pnpm environment before switching CI to frozen-lockfile mode.
