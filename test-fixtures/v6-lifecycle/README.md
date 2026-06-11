# V6 lifecycle fixtures

These fixtures describe backend lifecycle cases for the governed patch loop:

- rollback must load checkpoint by proposal_id, not frontend checkpoint_dir
- double apply must be rejected by lifecycle state
- double rollback must be rejected by lifecycle state
- wrong approval_id must fail apply
- manifest proposal_id mismatch must fail rollback
- backup path escape must fail manifest validation
- created files are deleted on rollback
- modified files are restored on rollback

The Rust tests in `crates/patch-engine` and `apps/desktop/src-tauri` encode the executable portions available without a full app harness.
