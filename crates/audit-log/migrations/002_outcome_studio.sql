CREATE TABLE IF NOT EXISTS initiatives (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  mode TEXT NOT NULL,
  title TEXT NOT NULL,
  source TEXT NOT NULL,
  status TEXT NOT NULL,
  resume_status TEXT,
  standing_mandate_id TEXT,
  active_spec_version INTEGER NOT NULL DEFAULT 1,
  active_worktree_id TEXT,
  autonomy_level INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_initiatives_session ON initiatives(session_id, updated_at);

CREATE TABLE IF NOT EXISTS business_contexts (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  category TEXT NOT NULL,
  sensitivity TEXT NOT NULL DEFAULT 'internal',
  payload_json TEXT NOT NULL,
  source TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  superseded_at TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS spec_versions (
  initiative_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  status TEXT NOT NULL,
  change_reason TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  frozen_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(initiative_id, version),
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS objectives (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS assumptions (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  impact_if_false TEXT NOT NULL,
  confidence REAL NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS constraints (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  kind TEXT NOT NULL,
  attributable_to TEXT NOT NULL,
  testable INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS requirements (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  required_evidence_json TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS architecture_decisions (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS ux_contracts (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  contract_json TEXT NOT NULL,
  prototype_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS dream_contracts (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  horizon TEXT NOT NULL,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS studio_tasks (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  status TEXT NOT NULL,
  assigned_role TEXT NOT NULL,
  iteration_count INTEGER NOT NULL DEFAULT 0,
  max_iterations INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id, spec_version) REFERENCES spec_versions(initiative_id, version)
);

CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  spec_version INTEGER NOT NULL,
  role TEXT NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT NOT NULL,
  profile_version INTEGER NOT NULL,
  context_bundle_id TEXT NOT NULL,
  status TEXT NOT NULL,
  parse_result TEXT,
  token_estimate INTEGER,
  error_summary TEXT,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  ended_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id),
  FOREIGN KEY(context_bundle_id) REFERENCES context_bundles(id)
);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  spec_version INTEGER NOT NULL,
  agent_run_id TEXT,
  role TEXT NOT NULL,
  artifact_type TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  content_sha256 TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  redacted_summary TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id),
  FOREIGN KEY(agent_run_id) REFERENCES agent_runs(id)
);

CREATE TABLE IF NOT EXISTS agent_beliefs (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  spec_version INTEGER NOT NULL,
  agent_run_id TEXT,
  role TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS alignment_questions (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  from_role TEXT NOT NULL,
  to_role TEXT NOT NULL,
  blocking INTEGER NOT NULL,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  answer_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  answered_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS verification_evidence (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  requirement_id TEXT NOT NULL,
  task_id TEXT,
  evidence_type TEXT NOT NULL,
  status TEXT NOT NULL,
  provenance TEXT NOT NULL,
  output_ref TEXT,
  content_sha256 TEXT NOT NULL,
  summary TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(requirement_id) REFERENCES requirements(id)
);

CREATE TABLE IF NOT EXISTS standing_mandates (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  payload_json TEXT NOT NULL,
  approved_by_source TEXT NOT NULL,
  approved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS governed_worktrees (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL UNIQUE,
  repo_root TEXT NOT NULL,
  worktree_path TEXT NOT NULL UNIQUE,
  branch_name TEXT NOT NULL UNIQUE,
  base_commit TEXT NOT NULL,
  status TEXT NOT NULL,
  approved_by_source TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  cleaned_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS orchestration_events (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  actor_role TEXT NOT NULL,
  kind TEXT NOT NULL,
  requirement_ids_json TEXT NOT NULL,
  adr_ids_json TEXT NOT NULL,
  assumption_ids_json TEXT NOT NULL,
  features_json TEXT NOT NULL,
  provenance TEXT NOT NULL,
  redacted_summary TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE INDEX IF NOT EXISTS idx_orchestration_events_timeline ON orchestration_events(initiative_id, created_at);

CREATE TABLE IF NOT EXISTS pulse_findings (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  kind TEXT NOT NULL,
  severity REAL NOT NULL,
  source TEXT NOT NULL,
  experimental INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS pulse_snapshots (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  observer_kind TEXT NOT NULL,
  model_version TEXT NOT NULL,
  model_checksum TEXT NOT NULL,
  calibrated INTEGER NOT NULL,
  state_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS interventions (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  task_id TEXT,
  kind TEXT NOT NULL,
  source_finding_id TEXT,
  status TEXT NOT NULL,
  rationale TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  resolved_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(source_finding_id) REFERENCES pulse_findings(id)
);

CREATE TABLE IF NOT EXISTS proof_reports (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL,
  spec_version INTEGER NOT NULL,
  report_json TEXT NOT NULL,
  content_sha256 TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id)
);

