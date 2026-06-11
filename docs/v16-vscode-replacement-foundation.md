# Synthesize v16: VS Code Replacement Foundation

Synthesize’s VS Code replacement goal is not to clone every VS Code feature first. The goal is to replace VS Code for workflows where local AI-governed coding is the primary loop.

V16 adds the first daily-editor fundamentals around Synthesize’s existing local-agent patch lifecycle:

- multi-tab editing,
- guarded save / save all / refresh from disk,
- guarded user file create/rename/delete,
- quick open,
- command palette,
- editor settings,
- Git stage/unstage/commit UI,
- existing project search, Git status, LSP scaffold, and governed task runner.

## Trust boundaries

V16 does not weaken the existing architecture:

- The model is untrusted.
- The frontend is not the authority for patch approval/apply.
- Agent patches remain backend-governed.
- Task execution remains backend-detected and approved.
- User file operations go through RepoGuard.
- Git mutations are user-initiated and audited.

## Not yet implemented

V16 is not full VS Code parity. Missing major pillars include:

- real LSP JSON-RPC client,
- diagnostics/hover/go-to-definition,
- debugger/DAP,
- extension ecosystem,
- remote development,
- full terminal parity,
- signed installers and auto-update.

## V17 recommendation

Make TypeScript LSP real:

- spawn/manage TypeScript language server,
- diagnostics panel,
- hover,
- go to definition,
- find references,
- symbols,
- format document.
