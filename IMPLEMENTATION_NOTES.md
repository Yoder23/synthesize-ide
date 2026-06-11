# Implementation Notes

This repo is intentionally production-shaped rather than demo-shaped. The main areas requiring full implementation are:

1. Tauri dialog and filesystem command wiring.
2. llama.cpp binary discovery/bundling.
3. Runtime process supervisor and streaming HTTP client.
4. Full unified-diff parser/apply engine.
5. Git checkpoint manager.
6. ripgrep/tree-sitter indexing worker.
7. Real sandbox backends:
   - macOS/Linux: Docker, bubblewrap, firejail, or restricted subprocess mode.
   - Windows: WSL/devcontainer/restricted child process mode.
8. Session replay UI backed by SQLite.

Do not weaken the architecture to move faster. Implement vertical slices through the trusted boundary.
