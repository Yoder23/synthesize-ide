# Autonomous App Foundry gap analysis

This audit describes the repository before the App Foundry correction. It is
not a completion report.

## Production behavior versus demonstration

`studio::dream_factory_tick` and `dream_factory::DreamFactoryController`
persist a one-concept factory state, select dependency-ready `studio_tasks`,
and create a governed worktree. `studio::run_dream_team` invokes the configured
Dreamer lane and the fixed Skeptic/FDE/UX/Architect/Planner list. Their Context
Capsules and AgentRuns are persisted by `orchestration_core::prepare_role_run`.

The manufacturing behavior is still a deterministic demonstration:
`studio::factory_build_and_review` writes a task Markdown file under
`dreams/<initiative>/`, regards the `Implementation verified.` marker as
validation evidence, and records synthetic Verifier/Reviewer/final-review
events. It is not a runnable application and does not invoke the configured
Builder, Verifier, Reviewer, final UX, or final FDE runtime.

## Specific gaps

* Builder: configured `run_configured_role` can validate and enqueue a typed
  patch through `apply_studio_operation`, but factory execution does not invoke
  it and the queue targets the Synthesize repository rather than a Dream app
  workspace.
* Verifier/Reviewer: factory execution records events and routes a synthetic
  verdict; neither configured runtime is invoked.
* Final UX/FDE: `dream_factory_tick` records hard-coded pass events.
* Planner: `materialize_authoritative_artifact` now creates real
  `studio_tasks` and `dream_task_dependencies`, but their defaults are Markdown
  task outputs, not application-scaffold tasks.
* Context: role capsules exist, but no persisted stage-input manifest proves
  every required upstream artifact ID/source hash per Product Council stage.
* Transitions/questions: `apply_studio_operation` persists `AskAgent` and
  `RequestTransition`; no controller worker reliably resumes/routs them.
* Workspace: `worktree_manager::WorktreeManager` protects a repository
  worktree; no backend-owned approved Dream output root or application manifest
  exists. Therefore Dream Mode changes a governed Synthesize worktree rather
  than creating a standalone app.
* Gallery/launch: no application manifest, preview server, launch check, or
  Dream Gallery candidate exists.

The former `dream-factory-operation.md` must be read as a fixture-flow guide,
not evidence of an autonomous app factory. No documentation should claim a
runnable Qwen-built application until the App Foundry acceptance flow passes.
