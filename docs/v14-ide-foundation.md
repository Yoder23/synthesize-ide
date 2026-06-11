# Synthesize v16 IDE foundation

Synthesize's replacement target is not "clone all of VS Code." The goal is to replace VS Code for the workflow where local AI-governed repo editing is the primary loop.

V14 adds the first IDE foundation around that loop:

- guarded project search,
- read-only Git status,
- LSP capability detection/status scaffolding,
- governed task execution for tests/builds,
- continued backend-governed patch lifecycle.

## LSP status

V14 detects likely language servers for TypeScript/JavaScript, Python, Rust, and Go. Full LSP JSON-RPC integration is documented as the next milestone. The UI is explicit that diagnostics/go-to-definition/hover are planned capabilities, not fully live in this build.

## Git status

V14 includes a read-only Git status panel. Stage/unstage/commit/branch operations are intentionally not added yet. Git actions that mutate repo state should be designed with the same backend approval/audit model as patch application.

## Project search

Search reads only allowed text-like files through the backend and skips denied paths. It is not a replacement for ripgrep-scale indexing yet, but it is useful for dogfooding.

## Governed tasks

V14 introduces code execution through a task runner, not a free shell.

A task must be:

1. detected or represented as argv,
2. classified by the backend command guard,
3. explicitly approved by the user,
4. run by the backend with a scrubbed environment,
5. timeout-bounded,
6. audited.

Network/destructive/blocked commands are refused. Synthesize does not claim OS-level network sandboxing.
