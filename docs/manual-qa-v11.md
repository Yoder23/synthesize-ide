# Manual QA v11

## Fake runtime

1. Start Synthesize.
2. Open fixture repo.
3. Select Fake runtime.
4. Ask for a small auth refresh change.
5. Verify exact context bundle appears.
6. Verify a propose_patch enters Diff Queue.
7. Validate, approve, apply.
8. Confirm editor refreshes.
9. Roll back.
10. Confirm file restores and Session Log shows context/runtime/patch events.

## Local model server

1. Start a local model server, e.g. llama.cpp server on `127.0.0.1:8080`.
2. Select Local model server preset.
3. Health check through backend.
4. Ask for a small change on a throwaway repo.
5. Confirm model returns strict JSON operations.
6. Validate/approve/apply/rollback through backend.

## Managed llama.cpp

1. Enter llama.cpp server binary path.
2. Enter local GGUF model path.
3. Start managed llama.cpp.
4. Health check `http://127.0.0.1:<port>/v1`.
5. Run the same patch QA loop.

Command suggestions must remain classification-only and never execute.
