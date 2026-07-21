# Data model

The existing per-repository `.synthesize/synthesize-audit.sqlite` database remains the persistence boundary. Ordered migrations add records without replacing existing sessions, proposals, approvals, checkpoints, commands, runtime requests, or context bundles.

Core relationships:

```text
session -> initiative -> spec version -> requirement -> task -> operation/evidence
                    |-> objective/assumption/constraint/non-goal/ADR/UX contract
                    |-> role run -> artifact/belief/question/finding
                    |-> mandate -> dream contract -> governed worktree
                    |-> orchestration event -> pulse finding/snapshot/intervention
```

JSON payload columns hold versioned, schema-validated structured documents. Frequently queried authority fields—binding IDs, lifecycle state, versions, hashes, timestamps, and human-approval provenance—remain explicit columns. Historical specs, artifacts, evidence, events, and transitions are append-only. Sensitive exact context stays in existing context bundles and is excluded from proof exports by default.

Identifiers use stable prefixes (`INIT`, `SPEC`, `REQ`, `TASK`, `ART`, `EVID`, `DREAM`, `MANDATE`, `WT`, `EVENT`, `PULSE`). Foreign keys and repository/session/initiative bindings are checked by repositories and command adapters.

