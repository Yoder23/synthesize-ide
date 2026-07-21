# Architecture

Synthesize is a local-first Tauri application with a React/TypeScript presentation layer and a Rust trust boundary. It supports Assist, Studio, and Dream without giving model output direct repository or lifecycle authority.

```text
React workspace
  Assist editor/chat/diff       Studio goal workspace       Dream inbox/mandates
              | typed Tauri commands; no authoritative client state
Rust application boundary
  existing Assist services     orchestration-core          pulse-engine
  agent-protocol               intent-ledger               context-os
  repo/command/patch guards     worktree-manager            audit-log + migrations
              | explicit policy, identity, version, and evidence checks
Canonical repository           governed sibling worktree   approved runtimes
```

## Core invariant

Repository files, model output, command output, and test output are untrusted. Models emit typed operations. Only the trusted backend may validate bindings, persist approval, execute an allowed effect, change a state machine, or append audit evidence.

Assist retains its existing patch validation, approval, checkpoint, apply, rollback, task, terminal, Git, runtime, and session-log services. Studio adds an outcome ledger above those primitives; it does not replace or weaken them.

## Outcome control plane

`intent-ledger` is the authority for initiatives, immutable specs, requirements, tasks, Dream contracts, mandates, evidence, beliefs, questions, events, operation hashes, and proof reports. The SQLite store is repository-local and upgraded through numbered forward migrations.

`orchestration-core` owns nine versioned role profiles, prompt/output contracts, exact role-context construction, serial scheduling, run lifecycle persistence, Studio routing, Fake Runtime acceptance fixtures, and the declarative prototype schema. Agents communicate through persisted artifacts, beliefs, and addressed questions. Private chain-of-thought is neither requested nor displayed.

`worktree-manager` isolates implementation and prototypes from the active branch. It requires a clean active checkout, current approved base, canonical repo/initiative binding, local-human approval, safe derived paths, and one worktree per initiative. It exposes bounded diff and guarded cleanup, not merge authority.

`pulse-engine` consumes persisted orchestration facts. Symbolic monitors and a deterministic elapsed-time rule observer are the production path. The optional liquid observer is calibrated, validated, experimental, and shadow-only; it has no authority.

## Runtime and context

Role runtime and model selection are configuration, never authorization. `context-os` owns runtime capabilities, nine role projections, deterministic staged retrieval, priority pruning, delta compilation, structured summaries, sufficiency checks, exact-message persistence, and pre-inference token enforcement. Remote/private-LAN endpoints still pass the existing backend endpoint-approval policy. See [Context Operating System](context-operating-system.md).

## Proof boundary

Every Studio artifact carries an operation ID, initiative, optional task, active spec version, role, source context bundle, reason, and expected outcome. The backend enforces the role permission matrix and stores a SHA-256 operation binding. Verification requires declared evidence. Proof export contains outcome classifications and provenance but excludes sensitive exact context by default.

See [Outcome-Governed Studio](outcome-governed-studio.md) and the repository-specific specification under `docs/specs/outcome-governed-studio/`.
