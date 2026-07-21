# Model Library

Synthesize includes a local Model Library for GGUF coding models.

Supported import path:

- `.gguf` model files for llama.cpp compatibility.

Import records store:

- display name,
- local path,
- format,
- runtime compatibility,
- file size,
- optional sha256 if enabled by the backend command.

## Bootstrap a local model without Ollama

Synthesize can bootstrap a local GGUF model and llama.cpp server binary using only PowerShell:

```powershell
./scripts/bootstrap-local-model.ps1 -Model smoke
./scripts/start-local-model.ps1
```

Then run the real local-model smoke check:

```powershell
./scripts/local-model-smoke.ps1
```

The `smoke` lane downloads Qwen2.5 Coder 0.5B Instruct GGUF Q4_K_M for fast proof. For a stronger local coding lane:

```powershell
./scripts/bootstrap-local-model.ps1 -Model coder-1.5b
```

Downloaded artifacts are stored under `.synthesize-runtime/`, which is ignored by Git. The bootstrap records a local SHA-256 for the downloaded GGUF in `.synthesize-runtime/local-model.json`.
