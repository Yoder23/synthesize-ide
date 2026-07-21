# Studio migration notes

SQLite schema migrations are forward-only and recorded in `schema_migrations`:

1. Existing audit/context/patch schema.
2. Outcome-governed Studio entities and relationships.
3. Role runtime configuration, proof operation links, Dream deduplication, and autonomy usage.
4. Runtime capability registry, token-aware bundle metadata, Context Capsules, typed context requests, and structured summaries.

`audit_log::init_schema` applies missing versions in order inside transactions and verifies a previously recorded version has the expected name. Re-running initialization is idempotent. Existing session and audit rows are preserved; tests cover legacy upgrade, ordering, idempotence, and reopening a file-backed database.

No destructive data rewrite or automatic down-migration is performed. Back up `.synthesize/synthesize-audit.sqlite` before manually altering a production checkout. Older binaries should not be used to mutate a database after a newer schema has been opened.
