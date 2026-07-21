//! Trusted application-workspace and offline-web scaffold primitives. Models
//! supply a bounded name/slug; this crate owns every filesystem location.
use audit_log::new_id;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FoundryError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("workspace policy: {0}")]
    Policy(String),
}
pub type Result<T> = std::result::Result<T, FoundryError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamTarget {
    NewApplication,
    ExistingRepositoryEnhancement,
    Experiment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationManifest {
    pub id: String,
    pub initiative_id: String,
    pub name: String,
    pub slug: String,
    pub application_kind: String,
    pub workspace_path: String,
    pub status: String,
    pub run_command: Option<String>,
    pub build_command: Option<String>,
    pub test_commands: Vec<String>,
    pub preview_url: Option<String>,
    pub entry_point: Option<String>,
    pub artifact_paths: Vec<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

pub struct ApplicationWorkspaceManager<'a> {
    conn: &'a Connection,
}
impl<'a> ApplicationWorkspaceManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn approve_output_root(&self, session: &str, root: &Path) -> Result<String> {
        fs::create_dir_all(root)?;
        let canonical = root.canonicalize()?;
        if canonical.components().count() < 2 {
            return Err(FoundryError::Policy("output root is too broad".into()));
        }
        let id = new_id("DREAM_ROOT");
        self.conn.execute("INSERT INTO dream_output_roots (id,session_id,canonical_path,approved_by_source) VALUES (?1,?2,?3,'local-user') ON CONFLICT(session_id,canonical_path) DO UPDATE SET enabled=1,updated_at=datetime('now')",params![id,session,canonical.to_string_lossy()])?;
        Ok(self.conn.query_row(
            "SELECT id FROM dream_output_roots WHERE session_id=?1 AND canonical_path=?2",
            params![session, canonical.to_string_lossy()],
            |r| r.get(0),
        )?)
    }
    pub fn create_offline_web_app(
        &self,
        session: &str,
        initiative: &str,
        root_id: &str,
        proposed_name: &str,
        proposed_slug: &str,
    ) -> Result<ApplicationManifest> {
        let root:String=self.conn.query_row("SELECT canonical_path FROM dream_output_roots WHERE id=?1 AND session_id=?2 AND enabled=1 AND approved_by_source='local-user'",params![root_id,session],|r|r.get(0)).optional()?.ok_or_else(||FoundryError::Policy("no approved Dream output root".into()))?;
        let slug = safe_slug(proposed_slug)?;
        let root = PathBuf::from(root).canonicalize()?;
        let path = root.join(&slug);
        if path.exists() {
            return Err(FoundryError::Policy(
                "application slug already exists; model cannot reuse a workspace".into(),
            ));
        }
        fs::create_dir_all(&path)?;
        let actual = path.canonicalize()?;
        if !actual.starts_with(&root) {
            return Err(FoundryError::Policy(
                "application workspace escaped output root".into(),
            ));
        }
        scaffold_offline_web(&actual, proposed_name)?;
        let id = new_id("APP");
        let manifest = ApplicationManifest {
            id: id.clone(),
            initiative_id: initiative.into(),
            name: proposed_name.trim().to_string(),
            slug,
            application_kind: "offline_web_v1".into(),
            workspace_path: actual.to_string_lossy().to_string(),
            status: "building".into(),
            run_command: Some(format!(
                "python -m http.server 4173 --directory \"{}\"",
                actual.to_string_lossy()
            )),
            build_command: Some("offline_web_v1 structural validation".into()),
            test_commands: vec!["offline_web_v1 structural validation".into()],
            preview_url: None,
            entry_point: Some("index.html".into()),
            artifact_paths: vec!["index.html".into(), "styles.css".into(), "app.js".into()],
            created_at: String::new(),
            completed_at: None,
        };
        self.conn.execute("INSERT INTO dream_applications (id,initiative_id,output_root_id,name,slug,application_kind,workspace_path,status,manifest_json) VALUES (?1,?2,?3,?4,?5,'offline_web_v1',?6,'building',?7)",params![manifest.id,initiative,root_id,manifest.name,manifest.slug,manifest.workspace_path,serde_json::to_string(&manifest).unwrap()])?;
        Ok(manifest)
    }
    pub fn validate_offline_web_app(&self, workspace: &Path) -> Result<()> {
        for file in ["index.html", "styles.css", "app.js", "README.md"] {
            if !workspace.join(file).is_file() {
                return Err(FoundryError::Policy(format!(
                    "missing scaffold file {file}"
                )));
            }
        }
        let html = fs::read_to_string(workspace.join("index.html"))?;
        let js = fs::read_to_string(workspace.join("app.js"))?;
        if html.contains("http://") || html.contains("https://") || js.contains("fetch(") {
            return Err(FoundryError::Policy(
                "offline scaffold contains remote dependency".into(),
            ));
        }
        if html.contains("Lorem ipsum") || html.contains("Generated implementation") {
            return Err(FoundryError::Policy(
                "placeholder application content is forbidden".into(),
            ));
        }
        Ok(())
    }
}
fn safe_slug(input: &str) -> Result<String> {
    let slug = input.trim().to_ascii_lowercase();
    if slug.len() < 3
        || slug.len() > 48
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || slug.starts_with('-')
        || slug.ends_with('-')
    {
        return Err(FoundryError::Policy("invalid application slug".into()));
    }
    Ok(slug)
}
fn scaffold_offline_web(root: &Path, name: &str) -> Result<()> {
    fs::write(root.join("index.html"),format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"styles.css\"></head><body><main id=\"app\"></main><script type=\"module\" src=\"app.js\"></script></body></html>",name))?;
    fs::write(root.join("styles.css"),"*{box-sizing:border-box}body{margin:0;font:16px system-ui;background:#101827;color:#edf2f7}main{max-width:900px;margin:auto;padding:3rem}button{font:inherit;padding:.6rem 1rem}\n")?;
    fs::write(root.join("app.js"),format!("const root=document.querySelector('#app');\nroot.innerHTML=`<section><h1>{}</h1><p id=\"status\">Ready to shape your day.</p><button id=\"start\">Start</button></section>`;\ndocument.querySelector('#start').addEventListener('click',()=>document.querySelector('#status').textContent='Session started.');\n",name))?;
    fs::write(
        root.join("README.md"),
        format!("# {}\n\nOffline web application scaffold.\n", name),
    )?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_contained_non_placeholder_offline_app() {
        let c = Connection::open_in_memory().unwrap();
        audit_log::init_schema(&c).unwrap();
        c.execute_batch(
            "PRAGMA foreign_keys=OFF; INSERT INTO sessions(id,repo_root) VALUES('s','repo');",
        )
        .unwrap();
        let base = std::env::temp_dir().join(new_id("foundry"));
        let m = ApplicationWorkspaceManager::new(&c);
        let root = m.approve_output_root("s", &base).unwrap();
        let app = m
            .create_offline_web_app("s", "i", &root, "Focus Garden", "focus-garden")
            .unwrap();
        m.validate_offline_web_app(Path::new(&app.workspace_path))
            .unwrap();
        let _ = fs::remove_dir_all(base);
    }
}
