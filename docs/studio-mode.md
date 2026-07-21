# Studio Mode operator guide

1. Open a repository and switch the mode selector to Studio.
2. Enter the desired outcome and create an initiative. Synthesize runs the bounded discovery, concept, challenge, UX, architecture, and planning fixtures and stops at scope approval.
3. Review Overview, Intent, UX, Architecture, and Plan. Approve scope explicitly.
4. Use Team to run a deterministic delivery scenario. The successful path records Builder, Verifier, and Reviewer runs and evidence. Negative scenarios exercise revision, replan, blocking questions, malformed output, permission violation, drift, and budget stops.
5. Inspect Pulse, Evidence, and Changes. Create a governed worktree only from a clean active checkout and the displayed approved base commit.
6. Export the privacy-safe proof report and perform the final Git review/merge yourself.

Backend controls pause/resume, lower autonomy, request alignment, and complete review. The UI never changes authoritative state locally.

## Recovery after interruption

Reopen the same canonical repository and session. The SQLite ledger in `.synthesize/synthesize-audit.sqlite` restores initiative state, active spec version, role runs, exact context bundles, questions, evidence, Pulse records, and worktree binding. Resume a paused initiative through the backend control. Prepared role runs left by a process interruption remain visible; rerun the role rather than pretending the prior invocation completed.

## Troubleshooting

- Scope approval refused: inspect incomplete requirements, high-impact assumptions, UX/ADR records, and the frozen spec.
- Role output refused: check role permission, schema, artifact size, active spec version, task binding, and context-bundle binding.
- Worktree refused: clean the active checkout, use the current HEAD as approved base, confirm the initiative belongs to this canonical repo, and ensure it has no existing worktree.
- Requirement will not verify: add every evidence type declared by the requirement.
- Review loop stopped: the task iteration budget was reached; replan or obtain a human decision.
- Runtime quality is poor: configure a better local or approved remote model for that role. Model quality depends on the configured runtime.

A clean branch and backups remain recommended. Synthesize does not claim OS-level sandboxing or complete network isolation.
