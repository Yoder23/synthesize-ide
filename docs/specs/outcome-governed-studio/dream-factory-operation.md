# Operating the Dream Factory

Dream Factory is a persisted, backend-owned, one-concept-at-a-time workflow.
The application must remain running while it is active. Closing it does not
discard state: the next start recovers a safe pending stage rather than
replaying an in-progress write.

## Running it

1. Open a clean Git repository in Synthesize.
2. Open Dream Workspace and configure the Dreamer (and optionally the
   specialist roles) to the desired OpenAI-compatible local endpoint, such as
   the Qwen Coder Ollama endpoint.
3. Select **Approve mandate & run one bounded cycle** for a single concept, or
   enable **Run Dream Factory continuously**. No Dream prompt is required.
4. Watch the factory state: concept, stage, active task, repair attempt,
   expected artifact, blocker, and completed concepts are backend records.

Every accepted concept receives a governed Git worktree automatically. Generated
prototype files are written below `dreams/<initiative-id>/` **inside that
worktree**, never in the repository's active checkout. Worktree paths and
checkpoints are visible through the initiative snapshot. Merge, push, release,
deployment, active-branch writes, network use, and package installation remain
human-only.

## Control flow

```text
Dreamer -> Skeptic -> FDE -> UX -> Architect -> Planner
  -> scope gate -> governed worktree -> dependency-ready task
  -> Builder -> deterministic validation -> Verifier/Reviewer
  -> repair or next task -> final UX -> final FDE -> candidate complete
  -> next Dream (only when continuous mode remains enabled)
```

The controller claims only one ready task whose dependencies are passed. A
patch is accepted only under the mandate-bound policy, within the task's
allowed path, with an audit event and rollback checkpoint. A failed validation
returns the same task to the repair loop; it never marks generated code as
complete merely because a file exists.

## Proven behavior

`studio::tests::dream_factory_completes_a_repaired_two_task_concept_then_starts_the_next`
is the deterministic end-to-end proof. It creates a Git repository, generates
a concept, materializes two dependency-linked tasks, creates a worktree,
forces validation failure and a Reviewer `REVISE`, repairs the task, passes both
tasks, completes final reviews, starts a second Dream, and proves the active
checkout commit did not change.

## Current boundary

The deterministic fixture path is the tested autonomous implementation path.
Configured Dreamer and advisory roles can use a Qwen/OpenAI-compatible endpoint;
real-model Builder patch execution still requires a separately validated local
smoke run before it can be represented as equivalent to the fixture path.
No claim of real-model autonomous completion should be made from fixture tests
alone.
