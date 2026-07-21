use audit_log::new_id;
use intent_ledger::{InitiativeMode, Ledger, Mandate};
use repo_guard::{FilePolicy, RepoGuard};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use thiserror::Error;

const MAX_GIT_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("ledger error: {0}")]
    Ledger(#[from] intent_ledger::LedgerError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git command failed: {0}")]
    Git(String),
    #[error("worktree safety check failed: {0}")]
    Safety(String),
    #[error("worktree binding rejected: {0}")]
    Binding(String),
    #[error("human approval required: {0}")]
    Approval(String),
}

pub type Result<T> = std::result::Result<T, WorktreeError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitState {
    pub repo_root: String,
    pub current_commit: String,
    pub branch: String,
    pub dirty: bool,
    pub status_porcelain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GovernedWorktree {
    pub id: String,
    pub initiative_id: String,
    pub repo_root: String,
    pub worktree_path: String,
    pub branch_name: String,
    pub base_commit: String,
    pub status: String,
    pub approved_by_source: String,
    pub created_at: String,
    pub cleaned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDiff {
    pub worktree_id: String,
    pub base_commit: String,
    pub head_commit: String,
    pub status_porcelain: String,
    pub diff: String,
    pub truncated: bool,
}

pub struct WorktreeManager<'a> {
    conn: &'a Connection,
    repo_root: PathBuf,
}

impl<'a> WorktreeManager<'a> {
    pub fn new(conn: &'a Connection, repo_root: impl AsRef<Path>) -> Self {
        Self {
            conn,
            repo_root: repo_root.as_ref().to_path_buf(),
        }
    }

    pub fn inspect(&self) -> Result<GitState> {
        let canonical = canonical_repo(&self.repo_root)?;
        let top = git_text(&canonical, &["rev-parse", "--show-toplevel"])?;
        let canonical_top = canonical_repo(Path::new(top.trim()))?;
        if canonical_top != canonical {
            return Err(WorktreeError::Safety(
                "Git top-level does not match opened repository".into(),
            ));
        }
        let commit = git_text(&canonical, &["rev-parse", "HEAD"])?;
        let branch = git_text(&canonical, &["branch", "--show-current"])?;
        let status = git_text(
            &canonical,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?;
        Ok(GitState {
            repo_root: canonical.to_string_lossy().to_string(),
            current_commit: commit.trim().into(),
            branch: branch.trim().into(),
            dirty: !status.trim().is_empty(),
            status_porcelain: status,
        })
    }

    pub fn create(
        &self,
        initiative_id: &str,
        approved_base_commit: &str,
        approved_by_source: &str,
    ) -> Result<GovernedWorktree> {
        if approved_by_source != "local-user"
            && approved_by_source != "mandate-bound-worktree-policy"
        {
            return Err(WorktreeError::Approval(
                "worktree creation requires local-user or mandate-bound policy approval".into(),
            ));
        }
        let state = self.inspect()?;
        if state.dirty {
            return Err(WorktreeError::Safety(
                "active working tree must be clean before isolated worktree creation".into(),
            ));
        }
        if state.current_commit != approved_base_commit {
            return Err(WorktreeError::Safety(format!(
                "approved base is stale: expected {}, current {}",
                approved_base_commit, state.current_commit
            )));
        }
        let ledger = Ledger::new(self.conn);
        let initiative = ledger.get_initiative(initiative_id)?;
        let canonical = canonical_repo(&self.repo_root)?;
        if canonical_path_string(&initiative.repo_root)? != canonical.to_string_lossy() {
            return Err(WorktreeError::Binding(
                "initiative belongs to a different repository".into(),
            ));
        }
        if matches!(
            initiative.mode,
            InitiativeMode::DreamIdeation
                | InitiativeMode::DreamPrototype
                | InitiativeMode::DreamIncubator
        ) && initiative.standing_mandate_id.is_none()
        {
            return Err(WorktreeError::Approval(
                "Dream worktree requires a standing mandate".into(),
            ));
        }
        if let Some(mandate_id) = &initiative.standing_mandate_id {
            let mandate_json: String = self.conn.query_row(
                "SELECT payload_json FROM standing_mandates WHERE id=?1 AND enabled=1 AND approved_by_source='local-user'",
                [mandate_id],
                |row| row.get(0),
            )?;
            let mandate: Mandate = serde_json::from_str(&mandate_json).map_err(|error| {
                WorktreeError::Safety(format!("invalid persisted mandate: {error}"))
            })?;
            mandate.validate()?;
            let used: i64 = self.conn.query_row(
                "SELECT prototypes_created FROM autonomy_usage WHERE initiative_id=?1",
                [initiative_id],
                |row| row.get(0),
            )?;
            if used >= i64::from(mandate.maximum_prototypes_per_cycle) {
                return Err(WorktreeError::Safety(format!(
                    "prototype budget exhausted ({used}/{})",
                    mandate.maximum_prototypes_per_cycle
                )));
            }
        }
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM governed_worktrees WHERE initiative_id=?1 AND status != 'cleaned'",
                [initiative_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Err(WorktreeError::Safety(
                "initiative already has an active governed worktree".into(),
            ));
        }

        let worktree_id = new_id("WT");
        let repo_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repo");
        let suffix = sanitize_identifier(initiative_id);
        let branch_name = format!("synthesize/{suffix}");
        let parent = canonical.parent().ok_or_else(|| {
            WorktreeError::Safety("repository has no safe parent directory".into())
        })?;
        let worktree_parent = parent.join(".synthesize-worktrees");
        fs::create_dir_all(&worktree_parent)?;
        let canonical_parent = worktree_parent.canonicalize()?;
        let worktree_path = canonical_parent.join(format!("{repo_name}-{suffix}"));
        if worktree_path.exists() {
            return Err(WorktreeError::Safety(
                "derived worktree path already exists; refusing path reuse".into(),
            ));
        }
        if git_branch_exists(&canonical, &branch_name)? {
            return Err(WorktreeError::Safety(
                "derived worktree branch already exists; refusing branch reuse".into(),
            ));
        }
        let worktree_git_arg = git_path_argument(&worktree_path);
        if let Err(error) = run_git(
            &canonical,
            &[
                "worktree",
                "add",
                "-b",
                &branch_name,
                &worktree_git_arg,
                approved_base_commit,
            ],
        ) {
            let _ = run_git(&canonical, &["branch", "-D", &branch_name]);
            return Err(error);
        }

        let created = (|| -> Result<GovernedWorktree> {
            let canonical_worktree = worktree_path.canonicalize()?;
            if !canonical_worktree.starts_with(&canonical_parent) {
                return Err(WorktreeError::Safety(
                    "derived worktree escaped its backend-owned parent".into(),
                ));
            }
            let _guard = RepoGuard::new(&canonical_worktree, FilePolicy::default())
                .map_err(|error| WorktreeError::Safety(error.to_string()))?;
            let worktree_top = canonical_repo(Path::new(
                git_text(&canonical_worktree, &["rev-parse", "--show-toplevel"])?.trim(),
            ))?;
            if worktree_top != canonical_worktree {
                return Err(WorktreeError::Binding(
                    "created path is not the expected Git worktree root".into(),
                ));
            }
            self.conn.execute(
                "INSERT INTO governed_worktrees
                 (id, initiative_id, repo_root, worktree_path, branch_name, base_commit, status, approved_by_source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
                params![
                    worktree_id,
                    initiative_id,
                    canonical.to_string_lossy(),
                    canonical_worktree.to_string_lossy(),
                    branch_name,
                    approved_base_commit,
                    approved_by_source
                ],
            )?;
            self.conn.execute(
                "UPDATE autonomy_usage SET prototypes_created=prototypes_created+1, updated_at=datetime('now')
                 WHERE initiative_id=?1",
                [initiative_id],
            )?;
            self.conn.execute(
                "UPDATE initiatives SET active_worktree_id=?2, updated_at=datetime('now') WHERE id=?1",
                params![initiative_id, worktree_id],
            )?;
            self.get(&worktree_id)
        })();
        if created.is_err() {
            let _ = run_git(
                &canonical,
                &["worktree", "remove", &git_path_argument(&worktree_path)],
            );
            let _ = run_git(&canonical, &["branch", "-D", &branch_name]);
        }
        created
    }

    pub fn get(&self, worktree_id: &str) -> Result<GovernedWorktree> {
        let row = self
            .conn
            .query_row(
                "SELECT id, initiative_id, repo_root, worktree_path, branch_name, base_commit,
                        status, approved_by_source, created_at, cleaned_at
                 FROM governed_worktrees WHERE id=?1",
                [worktree_id],
                |row| {
                    Ok(GovernedWorktree {
                        id: row.get(0)?,
                        initiative_id: row.get(1)?,
                        repo_root: row.get(2)?,
                        worktree_path: row.get(3)?,
                        branch_name: row.get(4)?,
                        base_commit: row.get(5)?,
                        status: row.get(6)?,
                        approved_by_source: row.get(7)?,
                        created_at: row.get(8)?,
                        cleaned_at: row.get(9)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| WorktreeError::Binding(format!("unknown worktree {worktree_id}")))?;
        self.validate_binding(&row)?;
        Ok(row)
    }

    pub fn candidate_diff(&self, worktree_id: &str) -> Result<CandidateDiff> {
        let worktree = self.get(worktree_id)?;
        if worktree.status != "active" {
            return Err(WorktreeError::Safety(
                "candidate diff requires an active worktree".into(),
            ));
        }
        let path = PathBuf::from(&worktree.worktree_path);
        let status = git_text(
            &path,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?;
        let head = git_text(&path, &["rev-parse", "HEAD"])?;
        let output = run_git(
            &path,
            &["diff", "--no-ext-diff", "--binary", &worktree.base_commit],
        )?;
        let mut diff_bytes = output.stdout;
        let truncated = diff_bytes.len() > MAX_GIT_OUTPUT_BYTES;
        diff_bytes.truncate(MAX_GIT_OUTPUT_BYTES);
        Ok(CandidateDiff {
            worktree_id: worktree.id,
            base_commit: worktree.base_commit,
            head_commit: head.trim().into(),
            status_porcelain: status,
            diff: String::from_utf8_lossy(&diff_bytes).to_string(),
            truncated,
        })
    }

    pub fn cleanup(&self, worktree_id: &str, confirmation_token: &str) -> Result<GovernedWorktree> {
        let worktree = self.get(worktree_id)?;
        if confirmation_token != format!("CLEANUP_WORKTREE:{worktree_id}") {
            return Err(WorktreeError::Approval(
                "cleanup confirmation token does not match backend worktree identity".into(),
            ));
        }
        if worktree.status != "active" {
            return Err(WorktreeError::Safety("worktree is not active".into()));
        }
        let path = PathBuf::from(&worktree.worktree_path);
        let status = git_text(
            &path,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )?;
        if !status.trim().is_empty() {
            return Err(WorktreeError::Safety(
                "worktree has uncommitted candidate changes; export/review them before cleanup"
                    .into(),
            ));
        }
        let canonical_repo = canonical_repo(&self.repo_root)?;
        run_git(
            &canonical_repo,
            &["worktree", "remove", &git_path_argument(&path)],
        )?;
        self.conn.execute(
            "UPDATE governed_worktrees SET status='cleaned', cleaned_at=datetime('now') WHERE id=?1 AND status='active'",
            [worktree_id],
        )?;
        self.conn.execute(
            "UPDATE initiatives SET active_worktree_id=NULL, updated_at=datetime('now')
             WHERE id=?1 AND active_worktree_id=?2",
            params![worktree.initiative_id, worktree_id],
        )?;
        self.get_without_live_validation(worktree_id)
    }

    fn validate_binding(&self, worktree: &GovernedWorktree) -> Result<()> {
        let canonical_active_repo = canonical_repo(&self.repo_root)?;
        if canonical_path_string(&worktree.repo_root)? != canonical_active_repo.to_string_lossy() {
            return Err(WorktreeError::Binding(
                "worktree record is bound to another repository".into(),
            ));
        }
        if worktree.status == "cleaned" {
            return Ok(());
        }
        let worktree_path = PathBuf::from(&worktree.worktree_path).canonicalize()?;
        let expected_parent = canonical_active_repo
            .parent()
            .ok_or_else(|| WorktreeError::Binding("repository has no parent".into()))?
            .join(".synthesize-worktrees")
            .canonicalize()?;
        if !worktree_path.starts_with(&expected_parent) {
            return Err(WorktreeError::Binding(
                "persisted worktree path escaped backend-owned parent".into(),
            ));
        }
        let top = canonical_repo(Path::new(
            git_text(&worktree_path, &["rev-parse", "--show-toplevel"])?.trim(),
        ))?;
        if top != worktree_path {
            return Err(WorktreeError::Binding(
                "worktree path substitution detected".into(),
            ));
        }
        Ok(())
    }

    fn get_without_live_validation(&self, worktree_id: &str) -> Result<GovernedWorktree> {
        self.conn
            .query_row(
                "SELECT id, initiative_id, repo_root, worktree_path, branch_name, base_commit,
                        status, approved_by_source, created_at, cleaned_at
                 FROM governed_worktrees WHERE id=?1",
                [worktree_id],
                |row| {
                    Ok(GovernedWorktree {
                        id: row.get(0)?,
                        initiative_id: row.get(1)?,
                        repo_root: row.get(2)?,
                        worktree_path: row.get(3)?,
                        branch_name: row.get(4)?,
                        base_commit: row.get(5)?,
                        status: row.get(6)?,
                        approved_by_source: row.get(7)?,
                        created_at: row.get(8)?,
                        cleaned_at: row.get(9)?,
                    })
                },
            )
            .map_err(Into::into)
    }
}

fn canonical_repo(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;
    if !canonical.join(".git").exists() {
        // Linked worktrees use a .git file, main worktrees normally use a directory.
        return Err(WorktreeError::Safety(format!(
            "{} is not a Git worktree",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn canonical_path_string(path: &str) -> Result<String> {
    Ok(Path::new(path)
        .canonicalize()?
        .to_string_lossy()
        .to_string())
}

fn sanitize_identifier(id: &str) -> String {
    let value: String = id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .map(|character| character.to_ascii_lowercase())
        .take(48)
        .collect();
    if value.is_empty() {
        "initiative".into()
    } else {
        value
    }
}

fn git_path_argument(path: &Path) -> String {
    let value = path.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn git_branch_exists(repo: &Path, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(WorktreeError::Git(
            String::from_utf8_lossy(&output.stderr).into(),
        )),
    }
}

fn git_text(repo: &Path, args: &[&str]) -> Result<String> {
    let output = run_git(repo, args)?;
    if output.stdout.len() > MAX_GIT_OUTPUT_BYTES {
        return Err(WorktreeError::Safety(
            "Git output exceeded bounded limit".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<Output> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(WorktreeError::Git(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audit_log::init_schema;
    use intent_ledger::{InitiativeMode, Ledger};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        conn: Connection,
        initiative_id: String,
        base: String,
    }

    fn fixture(label: &str) -> Fixture {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "synthesize-worktree-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init"]).unwrap();
        run_git(
            &root,
            &["config", "user.email", "synthesize@example.invalid"],
        )
        .unwrap();
        run_git(&root, &["config", "user.name", "Synthesize Test"]).unwrap();
        fs::write(root.join("README.md"), "base\n").unwrap();
        run_git(&root, &["add", "README.md"]).unwrap();
        run_git(&root, &["commit", "-m", "base"]).unwrap();
        let base = git_text(&root, &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let root_string = root.canonicalize().unwrap().to_string_lossy().to_string();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('s', ?1)",
            [&root_string],
        )
        .unwrap();
        let initiative = Ledger::new(&conn)
            .create_initiative(
                "s",
                &root_string,
                "Worktree",
                InitiativeMode::Studio,
                "user",
                None,
            )
            .unwrap();
        Fixture {
            root,
            conn,
            initiative_id: initiative.id,
            base,
        }
    }

    #[test]
    fn creates_bound_worktree_without_touching_active_branch() {
        let fixture = fixture("create");
        let active_before = fs::read_to_string(fixture.root.join("README.md")).unwrap();
        let manager = WorktreeManager::new(&fixture.conn, &fixture.root);
        let worktree = manager
            .create(&fixture.initiative_id, &fixture.base, "local-user")
            .unwrap();
        fs::write(
            Path::new(&worktree.worktree_path).join("README.md"),
            "candidate\n",
        )
        .unwrap();
        let candidate = manager.candidate_diff(&worktree.id).unwrap();
        assert!(candidate.diff.contains("candidate"));
        assert_eq!(
            fs::read_to_string(fixture.root.join("README.md")).unwrap(),
            active_before
        );
    }

    #[test]
    fn refuses_dirty_tree_stale_base_and_nonhuman_approval() {
        let dirty = fixture("dirty");
        fs::write(dirty.root.join("dirty.txt"), "dirty").unwrap();
        assert!(WorktreeManager::new(&dirty.conn, &dirty.root)
            .create(&dirty.initiative_id, &dirty.base, "local-user")
            .is_err());

        let stale = fixture("stale");
        assert!(WorktreeManager::new(&stale.conn, &stale.root)
            .create(&stale.initiative_id, "deadbeef", "local-user")
            .is_err());
        assert!(WorktreeManager::new(&stale.conn, &stale.root)
            .create(&stale.initiative_id, &stale.base, "builder")
            .is_err());
    }

    #[test]
    fn rejects_wrong_repository_binding_and_duplicates() {
        let first = fixture("binding-a");
        let second = fixture("binding-b");
        assert!(WorktreeManager::new(&second.conn, &second.root)
            .create(&first.initiative_id, &second.base, "local-user")
            .is_err());
        let manager = WorktreeManager::new(&first.conn, &first.root);
        let _ = manager
            .create(&first.initiative_id, &first.base, "local-user")
            .unwrap();
        assert!(manager
            .create(&first.initiative_id, &first.base, "local-user")
            .is_err());
    }

    #[test]
    fn cleanup_requires_exact_identity_and_clean_candidate() {
        let fixture = fixture("cleanup");
        let manager = WorktreeManager::new(&fixture.conn, &fixture.root);
        let worktree = manager
            .create(&fixture.initiative_id, &fixture.base, "local-user")
            .unwrap();
        assert!(manager
            .cleanup(&worktree.id, "CLEANUP_WORKTREE:wrong")
            .is_err());
        let cleaned = manager
            .cleanup(&worktree.id, &format!("CLEANUP_WORKTREE:{}", worktree.id))
            .unwrap();
        assert_eq!(cleaned.status, "cleaned");
        assert!(!Path::new(&worktree.worktree_path).exists());
    }
}
