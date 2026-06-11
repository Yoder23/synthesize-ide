# Architecture

Synthesize IDE is built around five layers:

```txt
UI Shell
  ├─ Monaco editor
  ├─ file explorer
  ├─ chat panel
  ├─ diff queue
  ├─ command approval
  ├─ runtime control center
  └─ session replay

Agent Harness
  ├─ agent profiles
  ├─ prompt compiler
  ├─ operation protocol
  ├─ schema validator
  ├─ context planner
  └─ policy router

Repo Intelligence
  ├─ file tree
  ├─ ripgrep search
  ├─ tree-sitter symbols
  ├─ import graph
  ├─ package scripts
  └─ context builder

Execution Layer
  ├─ repo guard
  ├─ patch engine
  ├─ command guard
  ├─ sandbox runner
  ├─ checkpoint manager
  └─ rollback manager

Trust Layer
  ├─ audit log
  ├─ context visibility
  ├─ model provenance
  ├─ patch provenance
  ├─ command provenance
  └─ privacy report
```

## Core invariant

Repository files, model outputs, test output, and command output are untrusted data. The trusted backend is the only component allowed to apply patches, read files through policy, execute commands, or write audit records.

## Product boundary

The product boundary is the operation protocol, not a particular model runtime. Any backend may generate text, but only operations passing schema validation and policy review can affect the repo.

## Runtime strategy

Default runtime is llama.cpp/GGUF because it provides a strong local default with a small operational footprint. Other runtimes are adapters behind the same contract:

- llama.cpp
- vLLM
- MLX
- Transformers/Python
- Ollama
- LM Studio
- OpenAI-compatible endpoint

## Vertical-slice plan

Each milestone must traverse UI, protocol, trusted backend, audit, and tests. Avoid building disconnected subsystems.

