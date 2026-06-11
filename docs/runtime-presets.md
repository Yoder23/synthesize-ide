# Runtime Presets

Synthesize includes local-first presets:

| Preset | Default URL | Protocol | Notes |
|---|---:|---|---|
| llama.cpp server | `http://localhost:8080/v1` | OpenAI-compatible local HTTP | Run GGUF models locally. |
| LM Studio local server | `http://localhost:1234/v1` | OpenAI-compatible local HTTP | Use LM Studio's local server mode. |
| Ollama local | `http://localhost:11434/v1` | OpenAI-compatible local HTTP route | Use Ollama's local compatibility route if available. |
| vLLM local server | `http://localhost:8000/v1` | OpenAI-compatible local HTTP | Workstation/server GPU setups. |
| Custom local server | `http://localhost:8080/v1` | OpenAI-compatible local HTTP | Bring your own local server. |

No cloud presets are included.
