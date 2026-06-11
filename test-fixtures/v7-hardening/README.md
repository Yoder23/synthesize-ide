# V7 hardening fixtures

V7 focuses on patch lifecycle correctness, not runtime breadth.

Covered in unit tests where Cargo is available:

- Created files in new nested directories pass `RepoGuard::resolve_for_write_path`.
- Created files in new nested directories are deleted during rollback.
- Approval insert + lifecycle transition roll back atomically if audit writing fails.
- Canonical repo-root comparison accepts equivalent path spellings.

Recommended CI/integration tests to add next:

- Simulated post-write DB failure during apply finalization restores from checkpoint and marks `apply_failed`.
- Simulated post-restore DB failure during rollback finalization marks `rollback_failed` without attempting to re-apply the patch.
- Multi-file write failure after an earlier file write restores all modified files.
