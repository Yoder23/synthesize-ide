# Runtime Contract

Synthesize IDE owns the runtime contract. Ollama, llama.cpp, vLLM, MLX, LM Studio, and OpenAI-compatible servers are adapters.

## Required capabilities

- List installed models.
- Import or install model.
- Load and unload model.
- Stream generation.
- Cancel generation.
- Report health.
- Benchmark model.
- Emit usage and errors.

## Why a service boundary

Inference backends can hang, crash, fragment VRAM, or require restart. The IDE UI must not share fate with a model runtime. Runtime adapters should launch or connect to subprocesses/services and communicate through a stable internal API.

