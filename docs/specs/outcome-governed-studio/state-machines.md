# State machines

## Initiative

Backend transition policy follows the master status vocabulary. Normal progression is created → discovery → concepting → challenging → UX design → architecture → planning → awaiting scope approval → implementing → verifying → reviewing → awaiting merge review → completed. Pause/resume preserves the prior resumable state. Blocked, failed, abandoned, and completed are terminal except for explicit recovery policy.

## Requirement

Proposed → approved → implementation started → implemented → verification pending → verified → outcome pending → outcome confirmed. Verification requires all mandatory evidence to exist and pass. Repairing, blocked, failed, rejected, superseded, and outcome disproven are explicit alternatives.

## Task and Dream

Task transitions are checked against the declared task lifecycle; iteration and elapsed/file budgets are enforced before automatic retries. Reviewer `REVISE` returns a task to a bounded repair state, `REPLAN` creates a new immutable spec version, `BLOCKED` pauses affected work, and `PASS` requires evidence.

Dream candidates follow proposed → deduplicated/challenged → shortlisted → prototype approved → prototyping → validated → promoted to goal, with rejected and archived alternatives. Worktree creation requires an enabled matching mandate and separate human approval.

## No-progress

Repeated identical blocking findings (three), oscillating results, repeated churn, unrelated paths, absent evidence gains, or budget exhaustion produce an explainable replan/pause/block intervention. There is no unbounded automatic loop.

