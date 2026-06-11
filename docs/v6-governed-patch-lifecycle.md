# V6 Governed Patch Lifecycle

Synthesize v6 makes the backend authoritative for the full patch lifecycle.

## Invariant

The frontend may display proposals and request actions, but it is not the authority for approval, apply, checkpoint identity, or rollback.

The model proposes typed operations. The backend persists a proposal snapshot, computes its canonical operation hash, validates the proposal, records approval, applies from the persisted snapshot, creates checkpoints, and records audit events.

## Lifecycle state machine

Legal transitions:

```text
proposed -> validated
proposed -> rejected
validated -> approved
approved -> applying
applying -> applied
applying -> apply_failed
applied -> rolling_back
rolling_back -> rolled_back
rolling_back -> rollback_failed
```

Illegal transitions are rejected by the backend helper. Examples:

```text
applied -> approved
rolled_back -> applied
rejected -> approved
applied -> applied
rolled_back -> rolled_back
```

## Backend-owned approval

Approval requires:

- persisted proposal in `validated` state
- matching `operation_sha256`
- backend-created approval record
- transition to `approved`
- audit event

## Backend-owned apply

Apply requires only:

- `repo_root`
- `proposal_id`
- `approval_id`

The frontend does not send patch contents during apply.

Backend apply behavior:

1. Acquire in-process repo mutation lock.
2. Load proposal snapshot and patch files from SQLite.
3. Verify proposal status is `approved`.
4. Verify approval record matches proposal and operation hash.
5. Transition `approved -> applying`.
6. Revalidate against current disk state.
7. Stage all file updates in memory.
8. Create checkpoint.
9. Write files.
10. Verify written files.
11. Persist checkpoint identity.
12. Transition `applying -> applied`.
13. Audit success or failure.

If apply fails, Synthesize attempts checkpoint restore and transitions to `apply_failed`.

## Backend-owned rollback

Rollback requires only:

- `repo_root`
- `proposal_id`

The frontend does not send checkpoint paths.

Backend rollback behavior:

1. Acquire in-process repo mutation lock.
2. Load applied proposal.
3. Load checkpoint ID from backend storage.
4. Verify checkpoint record matches proposal, repo root, and operation hash.
5. Derive checkpoint directory internally.
6. Validate checkpoint manifest.
7. Transition `applied -> rolling_back`.
8. Restore modified files and delete files created by the patch.
9. Verify restored/deleted state where practical.
10. Transition `rolling_back -> rolled_back`.
11. Audit success or failure.

## Checkpoint manifest validation

Rollback validates:

- manifest exists
- manifest proposal ID matches requested proposal
- manifest repo root matches requested repo
- backup paths do not escape checkpoint directory
- target paths resolve through RepoGuard
- created files do not have backup paths

## Mutation lock

V6 adds an in-process repo mutation lock for apply and rollback. Only one mutation may run per repo root in a single Synthesize process. Cross-process locking is not implemented yet.

## Known limitations

- Not OS-level atomic.
- Not cross-process locked.
- No real command sandbox.
- No command execution.
- Fake runtime only.
- llama.cpp/GGUF not implemented.
