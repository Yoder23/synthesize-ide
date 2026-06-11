# Synthesize v14 — IDE Foundation + Governed Task Runner

V14 moves Synthesize toward the VS Code replacement wedge: not by cloning every VS Code feature, but by adding the IDE foundations around the local governed agent loop.

## Added

- Read-only Git status panel.
- Guarded project search panel.
- Language Intelligence panel with LSP detection/status scaffolding for TypeScript/JavaScript, Python, Rust, and Go.
- Governed task runner:
  - detects common test/build tasks,
  - classifies commands with the backend command guard,
  - requires backend approval,
  - runs only argv-based commands,
  - scrubs environment,
  - enforces a 120-second timeout,
  - captures bounded stdout/stderr tails,
  - writes audit events.
- UI warning that task execution is governed and not a free terminal.
- Docs for v14 IDE replacement roadmap and governed tasks.

## Still intentionally not implemented

- Full LSP JSON-RPC client wiring.
- Full terminal/shell.
- Debug Adapter Protocol.
- VS Code extension compatibility.
- Remote SSH/devcontainer/WSL workflows.
- OS-level network sandboxing.

## Production boundary

Synthesize v14 supports governed code execution through approved tasks. It does not provide unrestricted terminal execution. Network/destructive/blocked commands remain refused by policy.
