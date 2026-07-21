# Studio manual QA checklist

Run `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `pnpm typecheck`, `pnpm test`, and `pnpm build` first.

- Open a repo; verify Assist still opens the editor and its existing Fake Runtime patch loop works.
- Switch to Studio; create a goal and verify every concept tab is populated and scope approval is required.
- Approve scope; run success, revision, replan, blocking-question, malformed-artifact, permission-violation, drift, and budget scenarios.
- Verify replan creates spec v2, old spec v1 remains frozen, and blocked questions are visible.
- Refresh/restart; confirm the initiative, timeline, exact role runs, evidence, and active version return.
- Verify Team shows all nine profiles and saves per-role runtime/model/timeout settings without bypassing endpoint approval.
- Verify prototype controls change local display state but cannot invoke Tauri, network, HTML, or scripts.
- Save an enabled Dream mandate; run two similar cycles and confirm candidate deduplication; reject one candidate.
- Approve a prototype and enable incubation only through explicit human actions; promote a candidate and confirm a full Studio initiative stops at scope approval.
- On a clean disposable Git repo, create a worktree at current HEAD and confirm the active branch/status do not change. Confirm dirty, stale-base, wrong-repo, nonhuman, duplicate, unsafe path, and wrong cleanup token requests fail.
- Inspect Pulse explanations and confirm experimental liquid output is labeled shadow-only and cannot apply an intervention.
- Export proof; verify requirement/evidence classifications and operation SHA-256 bindings are present, while exact context is excluded by default.
- Verify pause/resume and lower-autonomy controls are backend-persisted.
- Configure each role runtime with an explicit model window, output maximum, safety margin, conservative token method, structured-output behavior, source, and validation time.
- Expand a Team capsule and confirm role/task/spec/runtime bindings, the complete token equation, exact messages/hash, included and omitted records, summaries, and truncations are visible.
- Force a small model window and confirm inference is never contacted, the task becomes `BLOCKED_CONTEXT_OVERFLOW`, and the event recommends partitioning or narrowing.
- Have a configured fixture role emit a valid typed `request_context`; confirm RepoGuard rejects an escaping/denied path and an allowed request persists a new capsule before the same serialized role run continues.
- Change the active spec, approved ADR set, or runtime capability after capsule compilation and confirm the output is rejected as stale or binding-invalid.
- In Assist, open a file longer than 24,000 characters and confirm selected-file metadata says 24,000 included characters and shows the original length in a truncation record.
- Re-run the release gate after manual QA.
