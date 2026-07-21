CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  repo_root TEXT NOT NULL,
  git_commit_start TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  ended_at TEXT
);

CREATE TABLE IF NOT EXISTS audit_events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS context_bundles (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  token_estimate INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS patch_proposals (
  proposal_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  current_commit TEXT,
  operation_json TEXT NOT NULL,
  operation_sha256 TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('proposed','validated','rejected','approved','applying','applied','apply_failed','rolling_back','rolled_back','rollback_failed')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  validated_at TEXT,
  approved_at TEXT,
  applied_at TEXT,
  rolled_back_at TEXT,
  rejection_reason TEXT,
  checkpoint_id TEXT,
  checkpoint_dir TEXT,
  source_context_bundle_id TEXT,
  source_agent_profile_id TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  FOREIGN KEY(source_context_bundle_id) REFERENCES context_bundles(id)
);

CREATE TABLE IF NOT EXISTS patch_files (
  proposal_id TEXT NOT NULL,
  file_id TEXT NOT NULL,
  path TEXT NOT NULL,
  before_sha256 TEXT NOT NULL,
  unified_diff TEXT NOT NULL,
  diff_sha256 TEXT NOT NULL,
  risk TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(proposal_id, file_id),
  FOREIGN KEY(proposal_id) REFERENCES patch_proposals(proposal_id)
);


CREATE TABLE IF NOT EXISTS patch_checkpoints (
  checkpoint_id TEXT PRIMARY KEY,
  proposal_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  operation_sha256 TEXT NOT NULL,
  checkpoint_dir TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(proposal_id) REFERENCES patch_proposals(proposal_id)
);

CREATE TABLE IF NOT EXISTS patch_approvals (
  approval_id TEXT PRIMARY KEY,
  proposal_id TEXT NOT NULL,
  operation_sha256 TEXT NOT NULL,
  approved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  approved_by_source TEXT NOT NULL,
  approval_scope TEXT NOT NULL,
  FOREIGN KEY(proposal_id) REFERENCES patch_proposals(proposal_id)
);

CREATE TABLE IF NOT EXISTS task_snapshots (
  task_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  label TEXT NOT NULL,
  argv_json TEXT NOT NULL,
  cwd TEXT NOT NULL,
  risk TEXT NOT NULL,
  reason TEXT NOT NULL,
  requires_network INTEGER NOT NULL,
  may_modify_files INTEGER NOT NULL,
  detected_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(task_id, session_id, repo_root),
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS commands (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  task_id TEXT,
  repo_root TEXT,
  argv_json TEXT NOT NULL,
  cwd TEXT NOT NULL,
  risk TEXT NOT NULL,
  requires_network INTEGER NOT NULL DEFAULT 0,
  may_modify_files INTEGER NOT NULL DEFAULT 0,
  approved_at TEXT,
  started_at TEXT,
  finished_at TEXT,
  exit_code INTEGER,
  output_path TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS external_calls (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  destination TEXT NOT NULL,
  purpose TEXT NOT NULL,
  blocked INTEGER NOT NULL,
  timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);


CREATE TABLE IF NOT EXISTS endpoint_approvals (
  endpoint_url TEXT PRIMARY KEY,
  endpoint_classification TEXT NOT NULL,
  approved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  approved_by_source TEXT NOT NULL,
  allow_repo_context INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_requests (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  context_bundle_id TEXT,
  endpoint_url TEXT NOT NULL,
  endpoint_classification TEXT NOT NULL,
  model TEXT NOT NULL,
  provider TEXT NOT NULL,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  status TEXT NOT NULL,
  input_chars INTEGER NOT NULL DEFAULT 0,
  output_chars INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  FOREIGN KEY(context_bundle_id) REFERENCES context_bundles(id)
);

CREATE TABLE IF NOT EXISTS local_models (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  local_path TEXT NOT NULL,
  format TEXT NOT NULL,
  runtime_compatibility TEXT NOT NULL,
  size_bytes INTEGER,
  sha256 TEXT,
  imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_used_at TEXT
);

CREATE TABLE IF NOT EXISTS runtime_presets (
  id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  default_url TEXT NOT NULL,
  protocol TEXT NOT NULL,
  notes TEXT NOT NULL,
  local_by_default INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS managed_llamacpp_configs (
  id TEXT PRIMARY KEY,
  binary_path TEXT NOT NULL,
  model_path TEXT NOT NULL,
  host TEXT NOT NULL,
  port INTEGER NOT NULL,
  ctx_size INTEGER NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS skill_configs (
  id TEXT PRIMARY KEY,
  config_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
