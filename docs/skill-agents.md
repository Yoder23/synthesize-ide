# Skill Agents

Skill Agents let you run specialized coding agents (Qwen3 local lanes and optional cloud lanes) through a governed, serial queue.

## Production intent

- Use local Qwen3 skills for most work: low latency, predictable cost, no cloud dependency.
- Escalate only hard tasks to cloud-heavy skills.
- Keep one local model job running at a time to avoid GPU exhaustion.

## Queue model

- `skill_queue_spawn`: enqueue a skill task.
- `skill_queue_advance`: start the next queued task (only if none is running).
- `skill_queue_complete`: mark current task completed/failed/cancelled.
- `skill_queue_cancel_all`: cancel queued and running tasks.

The queue enforces a single running entry. This provides serial Qwen3 execution and deterministic hand-off behavior.

## Hand-off model

Each skill has:

- `allowed_operations`
- `allowed_hand_off_targets`
- `max_iterations`
- `model_registry_id`

A hand-off from skill A to skill B is accepted only when B is in A's `allowed_hand_off_targets` (or A's target list is intentionally empty for unrestricted mode).

## Recommended default graph

Use this graph for a production local-first loop:

```text
planner
  -> code-writer
  -> code-reviewer
  -> test-writer
  -> debugger
```

Optional cloud escalation:

```text
planner
  -> cloud-architect (heavy architecture)
  -> cloud-reasoner (hard correctness/security reasoning)
```

After cloud analysis, hand back to local skills for implementation and iteration.

## Configuration guidance

- Keep skill IDs stable and lowercase (`kebab-case`).
- Keep `max_iterations` conservative (5-15 for most skills).
- Keep allowed operations minimal per skill role.
- Define explicit hand-off targets for each skill to reduce accidental loops.
- Reserve cloud skills for bounded, high-value subtasks.

## Operational checklist

1. Confirm local runtime health (or cloud key + endpoint approval if escalating).
2. Spawn skill tasks with concise context summaries.
3. Advance queue and monitor current/queued/history views.
4. Review typed operations and route patches through validation/approval/apply.
5. Record outcomes in audit log and rollback proofs when needed.

## Failure handling

- If a skill fails, complete it with `failed` status and error message.
- Enqueue a debugger or reviewer skill as follow-up.
- Cancel all queued work if context changed materially (new branch, major file changes, task pivot).

## Security and trust boundary

Skill Agents do not bypass Synthesize governance.

- Models only propose typed operations.
- Backend owns validation, approval, apply, rollback, and audit.
- Non-local endpoints still require explicit endpoint approval before repo context leaves the machine.
