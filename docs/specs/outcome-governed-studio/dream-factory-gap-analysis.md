# Dream Factory gap analysis

## Current control flow (before corrective factory work)

The existing implementation is an inbox-oriented bounded ideation flow, not a
backend-owned autonomous product factory.

1. A cycle begins in `studio::dream_start_cycle` (`apps/desktop/src-tauri/src/studio.rs`). It creates a new `DreamIdeation` initiative, seeds minimal Context OS records, invokes Dreamer, then calls `run_dream_team`.
2. It stops after `run_dream_team` has invoked the fixed advisory list. It records a Dream Contract and returns; no controller owns a next stage or next concept.
3. `StudioWorkspace` previously used `setInterval` in its Dream effect to call `startDreamCycle`; that is frontend scheduling rather than durable orchestration.
4. `run_dream_team` invokes `Skeptic`, `Fde`, `UxDesigner`, `Architect`, and `Planner` sequentially. It falls back to the Dreamer runtime but treats each role error as an entry in a JSON result, allowing the surrounding cycle to continue.
5. Each invocation is prepared by `orchestration_core::prepare_role_run`, which compiles a role capsule. The current projections are role-policy based, but no stage manifest verifies that the required *specific upstream artifact IDs* exist before invocation.
6. `studio::apply_studio_operation` publishes most outputs through `Ledger::publish_artifact`. It materializes Dreamer contracts via `Ledger::create_dream`, but generic publication is not sufficient to advance a lifecycle.
7. Planner `task_graph` is stored as a generic artifact. It does not create executable `studio_tasks`, dependencies, or task requirements.
8. `AgentOperation::AskAgent` is persisted as an alignment question in `apply_studio_operation`; no controller enqueues the addressed role or resumes the requester.
9. `AgentOperation::RequestTransition` is validated/persisted as an event path in `apply_studio_operation`; it does not drive an initiative/controller transition.
10. Dream promotion in `studio::dream_action` changes initiative state around the Dream record. It does not preserve a complete immutable upstream lineage in a distinct implementation initiative because no full implementation lifecycle exists.
11. Manual `studio_run_role` chooses a user-selected role. The Dream helper chooses a fixed advisory list. Builder, Verifier, and Reviewer are not scheduled from ready tasks.
12. `StudioWorkspace::runRole` obtains `tasks[0]` for delivery roles. There is no dependency-aware task scheduler.
13. `governed_worktree_create` is a manual Tauri command backed by `WorktreeManager::create`; the Dream flow does not create a worktree automatically.
14. The next Dream is started by the frontend timer without waiting for terminal completion of the prior concept.

## Consequences

Current completion claims must not describe autonomous implementation, continuous
factory operation, automatic worktrees, dependency-aware execution, repair
loops, or backend-owned continuity. The existing code only proves mandate-bound
idea generation and advisory role calls.

## Corrective direction

The corrective implementation adds a persisted `DreamFactoryController` state
machine. It, rather than React, selects every next stage and task. Its stage
input manifests are verified against persisted Context Capsules. Materializers
turn accepted artifacts into objectives, assumptions, constraints, UX/ADR/spec
records, requirements, and dependency-linked tasks. The controller uses a
mandate-bound isolated-worktree policy for eligible automatic patches and only
begins the next concept after a terminal disposition.

## Implemented vertical slice and remaining audit boundary

The corrective implementation now persists factory/run state in migration 005,
uses `dream_factory::DreamFactoryController` for CAS stage changes and task
selection, materializes the Planner graph into `studio_tasks` plus
`dream_task_dependencies`, creates a governed worktree automatically, and
executes the tested fixture Builder/validation/Reviewer repair loop there.
`StudioWorkspace` only polls `dream_factory_tick`; it no longer starts ideas on
a timer. The exact flow and its real-model boundary are documented in
`dream-factory-operation.md`.

This document remains intentionally historical in its first section: it
describes the pre-correction gap rather than claiming that every future
real-model policy path is complete. In particular, a configured real Builder
requires the documented smoke validation before being described as equivalent
to the deterministic worktree path.
