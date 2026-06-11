# v11 Local Runtime Fixture Plan

These fixtures document the v11 manual/automated test target.

- Fake runtime end-to-end still validates/approves/applies/rolls back.
- Mock local model server returns strict JSON propose_patch.
- Runtime presets classify localhost as local.
- Non-local server requires backend approval.
- Managed llama.cpp config rejects missing binary/model and non-GGUF model files.
- GGUF import records metadata without downloading arbitrary files.
- Command suggestions remain classification-only.
