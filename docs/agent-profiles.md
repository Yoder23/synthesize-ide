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

## Studio roles

Every role has a versioned prompt contract, validated structured outputs, explicit permissions, and explicit prohibitions:

| Role | Owns | Cannot do |
| --- | --- | --- |
| Dreamer | Dream Contracts, novelty beliefs, reversible experiments | write/merge code or raise its autonomy |
| Forward-Deployed Engineer | objectives, outcome hypotheses, assumptions, constraints, non-goals | approve scope or implementation |
| UX Designer | UX Contracts, declarative prototypes, UX conformance | emit executable UI or code patches |
| Skeptic | disconfirmation experiments, findings, challenge verdicts | implement fixes or suppress evidence |
| Architect | alternatives, ADRs, architecture conformance | approve scope or merge |
| Planner | requirements, implementation spec, task graph, validation plan | edit repository files |
| Builder | task-bound patch proposals and implementation beliefs | widen scope, verify its own work, or merge |
| Verifier | verification evidence/verdicts and regression findings | change code or waive evidence |
| Reviewer | PASS/REVISE/REPLAN/BLOCKED verdicts | apply changes or mutate frozen specs |

Each invocation persists runtime/model/profile version, exact context-bundle ID, task/spec binding, terminal status, parse result, and a redacted error summary. Per-role runtime settings do not bypass endpoint approval or backend policy.
