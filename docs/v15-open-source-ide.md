# V15 Open-Source IDE Foundation

Synthesize v15 moves toward an open-source VS Code-replacement wedge for developers whose primary workflow is local AI-governed repo editing.

## What Synthesize tries to replace first

Synthesize does not try to clone every VS Code feature. It aims to replace VS Code for this loop:

```text
open repo
→ inspect/search code
→ ask a local coding agent
→ see exact context
→ review typed patch proposal
→ validate/approve/apply through backend
→ run governed tests/builds
→ audit/rollback
```

## Current IDE pillars

- Monaco editor shell
- Repo explorer
- Project Search
- Read-only Git status
- LSP detection/status scaffold
- Local model runtime control
- Agent profiles
- Context visibility
- Diff queue
- Governed task runner
- Session/audit log

## Next parity gaps

- Full TypeScript LSP client
- Diagnostics panel
- Git stage/commit/provenance UI
- Task failure → agent repair loop
- Full terminal/design for governed shell sessions
- Debug Adapter Protocol
- Remote development
- Extension/plugin ecosystem

## Open-source contribution priorities

1. Make the release gate green.
2. Harden task execution and diff application.
3. Add full TypeScript LSP.
4. Improve Git UX.
5. Add agent repair loop using governed task output.
