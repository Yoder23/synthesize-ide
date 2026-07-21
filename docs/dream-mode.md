# Dream Mode

Dream Mode generates forward-looking candidates under a local-user-approved standing mandate. It is an inbox and exploration workflow, not autonomous product authority.

Each bounded cycle now runs a sequential team pass after the Dreamer: Skeptic,
FDE, UX Designer, Architect, and Planner. Roles inherit the configured Dreamer
runtime when no specialist lane is saved, so one Qwen Coder configuration can
drive the whole team. The cycle returns the candidate and each handoff result;
continuous mode repeats this bounded sequence while the application remains
open.

## Standing mandate

A mandate is bound to one session and canonical repository and defines allowed Dream modes and paths; candidate, prototype, iteration, changed-file, and elapsed-time budgets; network and package-install policy; active-branch policy; and merge authority. Only `local-user` approval enables it. Default policy forbids network, package installation, active-branch writes, and autonomous merge.

## Lifecycle

Candidates are proposed, challenged, and shortlisted, then may be rejected, prototype-approved, prototyping, validated, promoted to a Studio goal, or archived. Similar candidates are deduplicated with a stable session-local semantic fingerprint. Counterarguments, assumptions, confidence, smallest experiment, cost, reversibility, and evidence are required fields.

Inbox actions are explicit human decisions:

- Reject or archive leaves the repository untouched.
- Approve prototype raises the initiative to `dream_prototype` only when the mandate permits it.
- Enable incubator raises it to `dream_incubator` only through a human action and permitted mandate.
- Promote creates a complete Studio discovery/specification snapshot and stops at scope approval.

Dream Mode does not merge autonomously. Prototype work uses a governed sibling worktree, never the active checkout. Continuous operation means repeated bounded cycles while the mandate is enabled and while the application is running; each cycle must still pass budgets, deduplication, and backend policy. It is not a daemon and does not continue after Synthesize exits.
