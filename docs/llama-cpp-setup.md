# llama.cpp Setup

Synthesize supports two llama.cpp paths.

## Manual server

Start llama.cpp server yourself with a GGUF coding model:

```bash
./llama-server --model /models/coder.gguf --host 127.0.0.1 --port 8080 --ctx-size 8192
```

Then configure Synthesize:

- Runtime mode: Local model server
- URL: `http://localhost:8080/v1`
- Model name: the model name expected by your server

## Managed llama.cpp

Synthesize can start a local llama.cpp server binary:

1. Bootstrap or import a local `.gguf` coding model.
2. Bootstrap or enter the path to the llama.cpp server binary.
3. Enter port and context size.
4. Click **Start managed llama.cpp**.
5. Run the backend health check.

Synthesize starts the process via argv-only process APIs and binds to `127.0.0.1` by default.

For a no-Ollama bootstrap:

```powershell
./scripts/bootstrap-local-model.ps1 -Model smoke
./scripts/start-local-model.ps1
./scripts/local-model-smoke.ps1
```

Use `-Model coder-1.5b` for a stronger local coding model once the smoke path works.


## v11.1 release-candidate notes

- Managed llama.cpp status now detects exited child processes.
- Managed llama.cpp stdout/stderr are consumed with bounded log tails to avoid pipe-buffer blocking.
- Local model/runtime metadata now uses an app-data directory instead of a temporary directory when possible.
- Agent Profile selection now affects the backend-generated local agent system prompt.
- Generate and commit `pnpm-lock.yaml` in a real pnpm environment before switching CI to frozen-lockfile mode.
