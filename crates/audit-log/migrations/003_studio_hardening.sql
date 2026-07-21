CREATE TABLE IF NOT EXISTS role_runtime_configs (
  session_id TEXT NOT NULL,
  role TEXT NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT NOT NULL,
  endpoint_url TEXT,
  timeout_seconds INTEGER NOT NULL DEFAULT 300,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(session_id, role),
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS operation_links (
  operation_id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  spec_version INTEGER NOT NULL,
  requirement_ids_json TEXT NOT NULL,
  adr_ids_json TEXT NOT NULL,
  context_bundle_id TEXT,
  operation_sha256 TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id),
  FOREIGN KEY(context_bundle_id) REFERENCES context_bundles(id)
);

CREATE TABLE IF NOT EXISTS dream_dedup_index (
  dream_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(dream_id) REFERENCES dream_contracts(id),
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  UNIQUE(session_id, fingerprint)
);

CREATE TABLE IF NOT EXISTS autonomy_usage (
  initiative_id TEXT PRIMARY KEY,
  candidates_created INTEGER NOT NULL DEFAULT 0,
  prototypes_created INTEGER NOT NULL DEFAULT 0,
  builder_iterations INTEGER NOT NULL DEFAULT 0,
  changed_files INTEGER NOT NULL DEFAULT 0,
  elapsed_minutes INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

