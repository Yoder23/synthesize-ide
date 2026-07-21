# Outcome-Governed Studio

Synthesize now has three product modes over one trusted backend:

- Assist is the original editor/chat/patch workflow.
- Studio turns a product goal into an approved, versioned specification, bounded implementation tasks, evidence, review, and a proof report.
- Dream explores mandate-bound opportunities without changing the active checkout. A human must explicitly raise autonomy or promote a candidate into Studio.

The invariant is unchanged: models propose typed operations; the backend validates bindings, permissions, transitions, budgets, evidence, and repository effects.

## Traceability model

An initiative owns immutable spec versions. A spec version connects objectives, assumptions, constraints, non-goals, requirements, architecture decisions, UX Contracts, tasks, role runs, artifacts, questions, beliefs, evidence, and operation hashes. A replan creates a new spec version; it never rewrites the frozen version that prior operations used.

Proof reports classify requirements as complete, incomplete, blocked, unverified, or outcome-pending. They include evidence provenance and operation-to-task/spec/context hashes, while excluding exact sensitive context by default.

## Studio lifecycle

`created -> discovery -> concepting -> challenging -> ux_design -> architecture -> planning -> awaiting_scope_approval -> implementing -> verifying -> reviewing -> awaiting_merge_review -> completed`

Pause/resume is backend-owned. Blocking questions and budget exhaustion route to `blocked`. Reviewer `REVISE` returns a task to a bounded revision loop; `REPLAN` invalidates the relevant assumption and creates a new immutable spec version; `PASS` advances only after required evidence exists. Agents may request transitions but cannot perform them.

## Requirement and task states

Requirements move through proposed, approved, implementation started, implemented, verification pending, verified, outcome pending, and outcome confirmed. Failed, blocked, repairing, deprecated, and superseded are explicit states. Verification is rejected until every declared evidence type has a passing record.

Tasks move through proposed, approved, ready, running, verifying, reviewing, passed, and completed, with bounded revising/replanning/blocked/cancelled branches. Scope contains allowlisted, expected, and forbidden relative paths.

## Modules

- `intent-ledger`: state machines, bindings, evidence gates, mandates, questions, beliefs, artifacts, operation hashes, snapshots, and proof reports.
- `orchestration-core`: nine role profiles, deterministic context broker, serial scheduling, runtime-run persistence, Fake Runtime scenarios, Studio bootstrap, and declarative prototype validation.
- `worktree-manager`: clean-tree/stale-base checks, sibling worktree creation, bounded diff inspection, and identity-confirmed cleanup.
- `pulse-engine`: symbolic monitors, production rule observer, experimental liquid observer validation, and advisory intervention routing.
- `audit-log`: ordered forward SQLite migrations and durable local records.

See [Studio Mode](studio-mode.md), [Dream Mode](dream-mode.md), [Pulse](pulse.md), [Declarative Prototypes](declarative-prototypes.md), and [Governed Worktrees](governed-worktrees.md).
