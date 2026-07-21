ALTER TABLE context_bundles ADD COLUMN input_token_count INTEGER;
ALTER TABLE context_bundles ADD COLUMN token_count_method TEXT NOT NULL DEFAULT 'legacy_character_count_not_tokens';

CREATE TABLE IF NOT EXISTS runtime_capabilities (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT NOT NULL,
  context_window_tokens INTEGER NOT NULL,
  maximum_output_tokens INTEGER NOT NULL,
  token_estimation_method TEXT NOT NULL,
  safety_margin_tokens INTEGER NOT NULL,
  structured_output_behavior TEXT NOT NULL,
  capability_source TEXT NOT NULL,
  last_validated_at TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(session_id, runtime, model),
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS context_capsules (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  role TEXT NOT NULL,
  agent_run_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT NOT NULL,
  context_window_tokens INTEGER NOT NULL,
  reserved_output_tokens INTEGER NOT NULL,
  safety_margin_tokens INTEGER NOT NULL,
  compiled_input_tokens INTEGER NOT NULL,
  token_count_kind TEXT NOT NULL,
  token_estimation_method TEXT NOT NULL,
  messages_sha256 TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id)
);

CREATE INDEX IF NOT EXISTS context_capsules_role_history
  ON context_capsules(initiative_id, role, task_id, created_at);

CREATE TABLE IF NOT EXISTS context_requests (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  role TEXT NOT NULL,
  source_capsule_id TEXT NOT NULL,
  resulting_capsule_id TEXT,
  status TEXT NOT NULL,
  request_json TEXT NOT NULL,
  rejection_reason TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id),
  FOREIGN KEY(source_capsule_id) REFERENCES context_capsules(id),
  FOREIGN KEY(resulting_capsule_id) REFERENCES context_capsules(id)
);

CREATE TABLE IF NOT EXISTS context_summaries (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  role TEXT,
  summary_type TEXT NOT NULL,
  summary_version INTEGER NOT NULL,
  source_version_start INTEGER NOT NULL,
  source_version_end INTEGER NOT NULL,
  source_ids_json TEXT NOT NULL,
  source_hashes_json TEXT NOT NULL,
  summary_json TEXT NOT NULL,
  omission_disclosure_json TEXT NOT NULL,
  status TEXT NOT NULL,
  generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  invalidated_at TEXT,
  UNIQUE(initiative_id, summary_type, task_id, role, summary_version),
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id)
);

CREATE INDEX IF NOT EXISTS context_summaries_valid
  ON context_summaries(initiative_id, summary_type, status);
