# Managed llama.cpp supervision

Synthesize includes a minimal managed llama.cpp supervisor.

## What it does

- Starts a local `llama-server` binary with argv-only process spawning.
- Uses a local `.gguf` model path.
- Binds to `127.0.0.1` by default.
- Tracks process status in memory.
- Calls `try_wait()` in status checks so exited processes are detected.
- Consumes stdout/stderr in bounded background readers to avoid pipe-buffer deadlocks.
- Keeps only the last 64KB of stdout and stderr for diagnostics.

## What it does not do

- It does not require Ollama or a Python model runtime.
- The bootstrap script can download a llama.cpp release binary and GGUF model into `.synthesize-runtime/`.
- The desktop supervisor itself still starts only an already-present local binary/model path.
- It does not provide OS-level sandboxing.
- It does not persist process state across app restarts.
- It does not expose arbitrary shell arguments in the default UI.

## Safety posture

Managed llama.cpp is intended as a local self-hosted model runtime path. It is not a general command execution feature. Command execution for repo tasks remains disabled.
