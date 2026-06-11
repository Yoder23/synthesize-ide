# V5 Backend-Owned Patch Approval

Synthesize's central invariant is unchanged:

> The model never acts. The model proposes typed operations. The trusted backend validates, records approval, applies changes transactionally, and audits everything.

## Proposal snapshotting

When the frontend sends a `propose_patch` operation to `validate_patch_proposal`, the backend:

1. Parses the typed operation.
2. Converts it to the patch engine representation.
3. Computes canonical JSON for the operation.
4. Computes `operation_sha256` over that canonical JSON.
5. Validates repo boundary and denied paths through `RepoGuard`.
6. Validates current commit if supplied.
7. Validates each `beforeSha256` against disk.
8. Validates unified-diff shape, hunk presence, and diff-header path agreement.
9. Rejects no-op patches.
10. Persists the proposal snapshot and patch file rows.
11. Emits an audit event.

The frontend may display the proposal, but the persisted snapshot is the source of truth.

## Duplicate proposal IDs

If `proposal_id` already exists, v5 rejects a new operation with a different `operation_sha256`. Exact duplicate hashes can be revalidated without silently overwriting a different patch body.

## Backend approval

The frontend approval button calls `approve_patch_proposal` with:

- `repo_root`
- `proposal_id`
- `operation_sha256`

The backend verifies that:

- the proposal exists;
- status is `validated`;
- the supplied hash matches the persisted snapshot.

Then it creates a `patch_approvals` row with:

- `approval_id`
- `proposal_id`
- `operation_sha256`
- `approved_at`
- `approved_by_source = local-user`
- `approval_scope = whole-proposal`

## Apply from approval only

`apply_approved_patch` no longer accepts a raw patch operation. It accepts only:

- `repo_root`
- `proposal_id`
- `approval_id`

The backend reloads the persisted operation and patch files, verifies the approval, re-runs validation against current disk state, checkpoints the repo, applies the patch, updates status, and emits an audit event.

## Transactional apply semantics

V5 uses checkpoint/restore transaction semantics:

1. Validate all paths and hashes first.
2. Read all original contents.
3. Compute all updated contents in memory.
4. Reject no-op files.
5. Create a checkpoint manifest.
6. Write all files.
7. If a write fails, attempt rollback from the checkpoint.
8. Mark the proposal failed and audit the failure.

This is not OS-level atomicity, but it avoids knowingly leaving partial mutation unrepaired.

## Checkpoint manifest

Checkpoint manifest entries contain:

- `path`
- `existed_before`
- `backup_path`
- `before_sha256`

Rollback behavior:

- If `existed_before = true`, restore from `backup_path`.
- If `existed_before = false`, delete the created file.
- Update proposal status to `rolled_back`.
- Emit an audit event.

## Disabled command execution

Command classification remains available as a planning/review aid. Guarded execution stays disabled in v5 until approval, sandbox, network, environment, and process-tree semantics are explicitly designed and enforced.
