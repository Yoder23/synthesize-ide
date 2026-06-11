# Local Model Server Runtime

Synthesize supports self-hosted local model servers through the Tauri backend.

Many local model servers expose an **OpenAI-compatible local HTTP** API. In Synthesize, this phrase means protocol shape only. It does not mean Synthesize uses OpenAI cloud, OpenAI accounts, or OpenAI API keys.

Examples:

- llama.cpp server: `http://localhost:8080/v1`
- LM Studio local server: `http://localhost:1234/v1`
- Ollama local route: `http://localhost:11434/v1` when available
- vLLM local server: `http://localhost:8000/v1`

Backend behavior:

1. Frontend asks backend to build a context bundle.
2. Backend persists exact model messages and `messages_sha256`.
3. Runtime generation receives only `context_bundle_id`.
4. Backend loads and verifies the persisted bundle.
5. Backend calls the local model server.
6. Model output is parsed as untrusted typed operations.

Non-local endpoints require explicit backend approval before repo context is sent.
