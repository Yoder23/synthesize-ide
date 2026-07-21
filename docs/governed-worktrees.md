# Governed worktrees and proof-carrying changes

Studio and approved Dream prototypes use a real Git sibling worktree derived from the canonical repository path and initiative identity. Creation is rejected unless the active checkout is clean, its HEAD equals the approved base commit, the request is locally human-approved, the initiative is bound to the same repository, and it has no active worktree. The active branch is never checked out or written by the worktree manager.

Candidate diffs are read-only, bounded, and inspected from the governed worktree. A patch/artifact operation is bound to its initiative, task, active spec version, requirement IDs, ADR IDs, exact source context bundle, and SHA-256 operation hash. Stale specs and duplicate operation IDs are rejected.

Cleanup requires the exact worktree identity confirmation token and a clean candidate worktree. Synthesize does not merge; final diff review and merge remain human Git operations. Worktrees are not an OS sandbox, and a clean branch plus backups are still recommended.
