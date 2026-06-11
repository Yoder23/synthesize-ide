# Agent Profiles

Synthesize separates model runtime from agent behavior.

- **Local Model Runtime**: how Synthesize talks to a local model server or managed llama.cpp process.
- **Agent Profile**: what behavior and operations are allowed.
- **Runtime Protocol**: local HTTP or fake fixture protocol.

Built-in profile concepts:

- **Fake Demo Agent**: deterministic fixture for QA.
- **Local Planner**: report/ask-user planning behavior.
- **Local Patcher**: may propose typed patches but cannot apply them.
- **Local Reviewer**: report-only review/critique behavior.

All patch approval, apply, checkpoint, rollback, and audit remain backend-owned.
