CREATE TABLE IF NOT EXISTS dream_factories (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  repo_root TEXT NOT NULL,
  mandate_id TEXT NOT NULL,
  status TEXT NOT NULL,
  current_initiative_id TEXT,
  completed_dream_count INTEGER NOT NULL DEFAULT 0,
  stop_after_current INTEGER NOT NULL DEFAULT 0,
  waiting_reason TEXT,
  lease_owner TEXT,
  lease_expires_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(session_id, repo_root),
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  FOREIGN KEY(current_initiative_id) REFERENCES initiatives(id)
);

CREATE TABLE IF NOT EXISTS dream_factory_runs (
  id TEXT PRIMARY KEY,
  factory_id TEXT NOT NULL,
  initiative_id TEXT NOT NULL,
  stage TEXT NOT NULL,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  active_task_id TEXT,
  expected_artifact TEXT,
  waiting_reason TEXT,
  idempotency_key TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(factory_id, idempotency_key),
  FOREIGN KEY(factory_id) REFERENCES dream_factories(id),
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(active_task_id) REFERENCES studio_tasks(id)
);

CREATE TABLE IF NOT EXISTS dream_task_dependencies (
  task_id TEXT NOT NULL,
  depends_on_task_id TEXT NOT NULL,
  PRIMARY KEY(task_id, depends_on_task_id),
  FOREIGN KEY(task_id) REFERENCES studio_tasks(id),
  FOREIGN KEY(depends_on_task_id) REFERENCES studio_tasks(id)
);

CREATE TABLE IF NOT EXISTS factory_stage_handoffs (
  id TEXT PRIMARY KEY,
  factory_run_id TEXT NOT NULL,
  stage TEXT NOT NULL,
  source_artifact_id TEXT NOT NULL,
  source_sha256 TEXT NOT NULL,
  context_capsule_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(factory_run_id, stage, source_artifact_id),
  FOREIGN KEY(factory_run_id) REFERENCES dream_factory_runs(id),
  FOREIGN KEY(context_capsule_id) REFERENCES context_capsules(id)
);
