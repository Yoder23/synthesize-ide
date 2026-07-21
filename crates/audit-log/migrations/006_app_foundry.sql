CREATE TABLE IF NOT EXISTS dream_output_roots (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  canonical_path TEXT NOT NULL,
  approved_by_source TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(session_id, canonical_path),
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS dream_applications (
  id TEXT PRIMARY KEY,
  initiative_id TEXT NOT NULL UNIQUE,
  output_root_id TEXT NOT NULL,
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  application_kind TEXT NOT NULL,
  workspace_path TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  FOREIGN KEY(initiative_id) REFERENCES initiatives(id),
  FOREIGN KEY(output_root_id) REFERENCES dream_output_roots(id)
);

ALTER TABLE dream_factories ADD COLUMN output_root_id TEXT REFERENCES dream_output_roots(id);
ALTER TABLE dream_factories ADD COLUMN dream_target TEXT NOT NULL DEFAULT 'new_application';
