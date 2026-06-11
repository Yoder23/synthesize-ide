# Local Model Runtime

Synthesize is designed for self-hosted open-source coding models. It does not require an OpenAI account or OpenAI API key.

Runtime modes:

- **Fake runtime**: deterministic fixture model for testing the governed patch loop.
- **Local model server**: a user-run local server using the OpenAI-compatible local HTTP protocol.
- **Managed llama.cpp**: Synthesize starts a user-provided llama.cpp server binary with a user-provided GGUF model path.

“OpenAI-compatible” in Synthesize means protocol shape only. It is a common local HTTP API implemented by llama.cpp server, LM Studio, vLLM, and some Ollama routes. It does not mean Synthesize uses OpenAI cloud.

Non-local model servers require explicit backend approval before Synthesize sends repo context.


## v11.1 release-candidate notes

- Managed llama.cpp status now detects exited child processes.
- Managed llama.cpp stdout/stderr are consumed with bounded log tails to avoid pipe-buffer blocking.
- Local model/runtime metadata now uses an app-data directory instead of a temporary directory when possible.
- Agent Profile selection now affects the backend-generated local agent system prompt.
- Generate and commit `pnpm-lock.yaml` in a real pnpm environment before switching CI to frozen-lockfile mode.
