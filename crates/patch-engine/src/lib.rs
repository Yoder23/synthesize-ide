use repo_guard::{FilePolicy, RepoGuard, RepoGuardError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("file hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch { path: String, expected: String, actual: String },
    #[error("commit mismatch: expected {expected}, got {actual}")]
    CommitMismatch { expected: String, actual: String },
    #[error("patch rejected: {0}")]
    Rejected(String),
    #[error("patch did not apply: {0}")]
    Apply(String),
    #[error("rollback failed: {0}")]
    Rollback(String),
    #[error("repo guard error: {0}")]
    RepoGuard(#[from] RepoGuardError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct PatchFile {
    pub id: String,
    pub path: String,
    pub before_sha256: String,
    pub unified_diff: String,
}

#[derive(Debug, Clone)]
pub struct PatchProposal {
    pub id: String,
    pub base_commit: Option<String>,
    pub current_commit: Option<String>,
    pub files: Vec<PatchFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchApprovalState {
    Proposed,
    UnderReview,
    Approved,
    PartiallyApproved,
    Rejected,
    Applied,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchRisk {
    Low,
    Medium,
    High,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct ValidatedPatchFile {
    pub id: String,
    pub path: String,
    pub real_path: PathBuf,
    pub risk: PatchRisk,
    pub existed_before: bool,
    pub before_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppliedPatchFile {
    pub id: String,
    pub path: String,
    pub after_sha256: String,
}

#[derive(Debug, Clone)]
pub struct PatchApplyResult {
    pub proposal_id: String,
    pub checkpoint_dir: PathBuf,
    pub checkpoint_id: String,
    pub applied_files: Vec<AppliedPatchFile>,
}

#[derive(Debug, Clone)]
pub struct RollbackResult {
    pub checkpoint_dir: PathBuf,
    pub checkpoint_id: String,
    pub proposal_id: Option<String>,
    pub restored_paths: Vec<String>,
    pub deleted_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHeader {
    pub old_path: String,
    pub new_path: String,
    pub has_old_marker: bool,
    pub has_new_marker: bool,
    pub hunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointManifest {
    pub version: u32,
    pub checkpoint_id: String,
    pub repo_root: String,
    pub proposal_id: String,
    pub created_at: String,
    pub files: Vec<CheckpointFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointFileEntry {
    pub path: String,
    pub existed_before: bool,
    pub backup_path: Option<String>,
    pub before_sha256: Option<String>,
}

struct StagedFile {
    patch_file: PatchFile,
    validated: ValidatedPatchFile,
    original: String,
    updated: String,
}

pub fn sha256_file(path: &Path) -> Result<String, PatchError> {
    let bytes = fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn sha256_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

pub fn validate_before_hash(real_path: &Path, expected: &str, display_path: &str) -> Result<(), PatchError> {
    let actual = sha256_file(real_path)?;
    if actual != expected {
        return Err(PatchError::HashMismatch { path: display_path.to_string(), expected: expected.to_string(), actual });
    }
    Ok(())
}

pub fn validate_current_commit(expected: Option<&str>, actual: Option<&str>) -> Result<(), PatchError> {
    match (expected, actual) {
        (Some(expected), Some(actual)) if expected != actual => Err(PatchError::CommitMismatch { expected: expected.to_string(), actual: actual.to_string() }),
        (Some(expected), None) if !expected.trim().is_empty() => Err(PatchError::CommitMismatch { expected: expected.to_string(), actual: "<non-git-repo>".into() }),
        _ => Ok(()),
    }
}

pub fn validate_patch_file_shape(file: &PatchFile) -> Result<PatchRisk, PatchError> {
    if file.id.trim().is_empty() {
        return Err(PatchError::Rejected("patch file id is required".into()));
    }
    if file.path.trim().is_empty() {
        return Err(PatchError::Rejected("patch file path is required".into()));
    }
    if file.before_sha256.trim().is_empty() {
        return Err(PatchError::Rejected(format!("beforeSha256 is required for {}", file.path)));
    }
    let header = parse_diff_header(&file.unified_diff)?;
    validate_diff_paths_match(&file.path, &header)?;
    Ok(classify_patch(&file.path, &file.unified_diff))
}

pub fn validate_patch_file_against_repo(repo_root: &Path, file: &PatchFile) -> Result<ValidatedPatchFile, PatchError> {
    let risk = validate_patch_file_shape(file)?;
    if risk == PatchRisk::Blocked {
        return Err(PatchError::Rejected(format!("{} is blocked by patch risk policy", file.path)));
    }
    let guard = RepoGuard::new(repo_root, FilePolicy::default())?;
    let real_path = guard.resolve_for_write_path(&file.path)?;
    let existed_before = real_path.exists();
    let header = parse_diff_header(&file.unified_diff)?;
    if existed_before && normalize_patch_path(&header.old_path) == "/dev/null" {
        return Err(PatchError::Rejected(format!("{} already exists but diff uses /dev/null as old path", file.path)));
    }
    if !existed_before && normalize_patch_path(&header.old_path) != "/dev/null" {
        return Err(PatchError::Rejected(format!("{} does not exist; creation diffs must use --- /dev/null", file.path)));
    }
    let before_sha256 = if existed_before {
        validate_before_hash(&real_path, &file.before_sha256, &file.path)?;
        Some(file.before_sha256.clone())
    } else {
        let empty_sha = sha256_text("");
        if file.before_sha256 != empty_sha {
            return Err(PatchError::HashMismatch { path: file.path.clone(), expected: empty_sha, actual: file.before_sha256.clone() });
        }
        None
    };
    let original = if existed_before { fs::read_to_string(&real_path)? } else { String::new() };
    let updated = apply_unified_diff_to_text(&original, &file.unified_diff)?;
    if updated == original {
        return Err(PatchError::Rejected(format!("patch for {} produced no content change", file.path)));
    }
    Ok(ValidatedPatchFile { id: file.id.clone(), path: file.path.clone(), real_path, risk, existed_before, before_sha256 })
}

pub fn validate_patch_proposal_against_repo(repo_root: &Path, proposal: &PatchProposal, actual_commit: Option<&str>) -> Result<Vec<ValidatedPatchFile>, PatchError> {
    if proposal.id.trim().is_empty() {
        return Err(PatchError::Rejected("proposal id is required".into()));
    }
    if proposal.files.is_empty() {
        return Err(PatchError::Rejected("proposal must contain at least one file".into()));
    }
    validate_current_commit(proposal.current_commit.as_deref(), actual_commit)?;
    let mut seen_paths = std::collections::BTreeSet::new();
    let mut seen_ids = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for file in &proposal.files {
        if !seen_ids.insert(file.id.clone()) {
            return Err(PatchError::Rejected(format!("duplicate patch file id {}", file.id)));
        }
        if !seen_paths.insert(normalize_patch_path(&file.path)) {
            return Err(PatchError::Rejected(format!("duplicate patch target path {}", file.path)));
        }
        out.push(validate_patch_file_against_repo(repo_root, file)?);
    }
    Ok(out)
}

pub fn apply_patch_proposal_transactional(repo_root: &Path, proposal: &PatchProposal, actual_commit: Option<&str>) -> Result<PatchApplyResult, PatchError> {
    // Revalidate against current disk state immediately before staging.
    let validated = validate_patch_proposal_against_repo(repo_root, proposal, actual_commit)?;
    let staged = stage_all_files(proposal, &validated)?;

    // Checkpoint before any write. This is checkpoint/restore transactionality, not OS atomicity.
    let checkpoint_dir = create_checkpoint(repo_root, &proposal.id, &validated)?;
    let checkpoint_id = checkpoint_id_from_dir(&checkpoint_dir);

    if let Err(err) = write_staged_files(&staged) {
        let restore = rollback_checkpoint_for_proposal(repo_root, &checkpoint_id, &proposal.id);
        return Err(PatchError::Apply(format!("write failed; checkpoint restore attempted: {}; restore_result={:?}", err, restore.err())));
    }

    let mut applied_files = Vec::new();
    for staged_file in &staged {
        match sha256_file(&staged_file.validated.real_path) {
            Ok(after_sha256) => {
                let expected_after = sha256_text(&staged_file.updated);
                if after_sha256 != expected_after {
                    let restore = rollback_checkpoint_for_proposal(repo_root, &checkpoint_id, &proposal.id);
                    return Err(PatchError::Apply(format!("post-write verification hash mismatch for {}; checkpoint restore attempted; restore_result={:?}", staged_file.patch_file.path, restore.err())));
                }
                applied_files.push(AppliedPatchFile {
                    id: staged_file.patch_file.id.clone(),
                    path: staged_file.patch_file.path.clone(),
                    after_sha256,
                });
            }
            Err(err) => {
                let restore = rollback_checkpoint_for_proposal(repo_root, &checkpoint_id, &proposal.id);
                return Err(PatchError::Apply(format!("post-write verification failed; checkpoint restore attempted: {}; restore_result={:?}", err, restore.err())));
            }
        }
    }
    Ok(PatchApplyResult { proposal_id: proposal.id.clone(), checkpoint_dir, checkpoint_id, applied_files })
}

/// Backward-compatible wrapper; Tauri apply uses `apply_patch_proposal_transactional` from persisted snapshots.
pub fn apply_patch_proposal(repo_root: &Path, proposal: &PatchProposal, actual_commit: Option<&str>) -> Result<PatchApplyResult, PatchError> {
    apply_patch_proposal_transactional(repo_root, proposal, actual_commit)
}

fn stage_all_files(proposal: &PatchProposal, validated: &[ValidatedPatchFile]) -> Result<Vec<StagedFile>, PatchError> {
    let mut staged = Vec::new();
    for file in &proposal.files {
        let target = validated.iter().find(|v| v.id == file.id).ok_or_else(|| PatchError::Apply(format!("validated file missing for {}", file.id)))?;
        if target.existed_before {
            validate_before_hash(&target.real_path, &file.before_sha256, &file.path)?;
        }
        let original = if target.real_path.exists() { fs::read_to_string(&target.real_path)? } else { String::new() };
        let updated = apply_unified_diff_to_text(&original, &file.unified_diff)?;
        if updated == original {
            return Err(PatchError::Apply(format!("patch for {} produced no content change", file.path)));
        }
        staged.push(StagedFile { patch_file: file.clone(), validated: target.clone(), original, updated });
    }
    Ok(staged)
}

fn write_staged_files(staged: &[StagedFile]) -> Result<(), PatchError> {
    for file in staged {
        if let Some(parent) = file.validated.real_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file.validated.real_path, file.updated.as_bytes())?;
    }
    Ok(())
}

pub fn create_checkpoint(repo_root: &Path, proposal_id: &str, files: &[ValidatedPatchFile]) -> Result<PathBuf, PatchError> {
    let checkpoint_id = format!("{}-{}", timestamp(), sanitize_id(proposal_id));
    let checkpoint_dir = repo_root.join(".synthesize").join("checkpoints").join(&checkpoint_id);
    let backup_root = checkpoint_dir.join("backups");
    fs::create_dir_all(&backup_root)?;

    let mut manifest_files = Vec::new();
    for file in files {
        let backup_path = if file.existed_before {
            let relative_backup = format!("backups/{}.bak", sanitize_id(&file.id));
            let backup_abs = checkpoint_dir.join(&relative_backup);
            fs::copy(&file.real_path, &backup_abs)?;
            Some(relative_backup)
        } else {
            None
        };
        manifest_files.push(CheckpointFileEntry {
            path: file.path.clone(),
            existed_before: file.existed_before,
            backup_path,
            before_sha256: file.before_sha256.clone(),
        });
    }

    let manifest = CheckpointManifest {
        version: 2,
        checkpoint_id: checkpoint_id.clone(),
        repo_root: repo_root.canonicalize().unwrap_or_else(|_| repo_root.to_path_buf()).to_string_lossy().to_string(),
        proposal_id: proposal_id.to_string(),
        created_at: timestamp(),
        files: manifest_files,
    };
    fs::write(checkpoint_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;
    Ok(checkpoint_dir)
}

pub fn rollback_checkpoint_for_proposal(repo_root: &Path, checkpoint_id: &str, proposal_id: &str) -> Result<RollbackResult, PatchError> {
    if checkpoint_id.trim().is_empty() || checkpoint_id.contains('/') || checkpoint_id.contains('\\') || checkpoint_id.contains("..") {
        return Err(PatchError::Rollback("invalid checkpoint id".into()));
    }
    let checkpoint_dir = repo_root.join(".synthesize").join("checkpoints").join(checkpoint_id);
    rollback_checkpoint_internal(repo_root, &checkpoint_dir, Some(proposal_id))
}

/// Internal compatibility helper for tests and repair paths. Product rollback must prefer
/// `rollback_checkpoint_for_proposal` so callers never provide a checkpoint directory.
pub fn rollback_checkpoint(repo_root: &Path, checkpoint_dir: &Path) -> Result<RollbackResult, PatchError> {
    rollback_checkpoint_internal(repo_root, checkpoint_dir, None)
}

fn rollback_checkpoint_internal(repo_root: &Path, checkpoint_dir: &Path, expected_proposal_id: Option<&str>) -> Result<RollbackResult, PatchError> {
    let checkpoint = if checkpoint_dir.is_absolute() { checkpoint_dir.to_path_buf() } else { repo_root.join(checkpoint_dir) };
    let guard = RepoGuard::new(repo_root, FilePolicy::default())?;
    let checkpoint_root = repo_root.join(".synthesize").join("checkpoints");
    let canonical_checkpoint = checkpoint.canonicalize()?;
    let canonical_checkpoint_root = checkpoint_root.canonicalize()?;
    if !canonical_checkpoint.starts_with(&canonical_checkpoint_root) {
        return Err(PatchError::Rollback("checkpoint path is outside .synthesize/checkpoints".into()));
    }
    let manifest_text = fs::read_to_string(canonical_checkpoint.join("manifest.json"))?;
    let manifest: CheckpointManifest = serde_json::from_str(&manifest_text)?;
    validate_checkpoint_manifest(repo_root, &canonical_checkpoint, &manifest, expected_proposal_id)?;

    let mut restored_paths = Vec::new();
    let mut deleted_paths = Vec::new();

    for entry in &manifest.files {
        let target = guard.resolve_for_write_path(&entry.path)?;
        if entry.existed_before {
            let rel_backup = entry.backup_path.as_ref().ok_or_else(|| PatchError::Rollback(format!("missing backup path for {}", entry.path)))?;
            let backup_abs = canonical_checkpoint.join(rel_backup);
            if !backup_abs.exists() {
                return Err(PatchError::Rollback(format!("backup path missing for {}", entry.path)));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&backup_abs, &target)?;
            if let Some(expected) = &entry.before_sha256 {
                let actual = sha256_file(&target)?;
                if &actual != expected {
                    return Err(PatchError::Rollback(format!("restored hash mismatch for {}", entry.path)));
                }
            }
            restored_paths.push(entry.path.clone());
        } else if target.exists() {
            fs::remove_file(&target)?;
            if target.exists() {
                return Err(PatchError::Rollback(format!("failed to delete created file {}", entry.path)));
            }
            deleted_paths.push(entry.path.clone());
        }
    }

    Ok(RollbackResult { checkpoint_dir: canonical_checkpoint, checkpoint_id: manifest.checkpoint_id, proposal_id: Some(manifest.proposal_id), restored_paths, deleted_paths })
}

pub fn validate_checkpoint_manifest(repo_root: &Path, canonical_checkpoint: &Path, manifest: &CheckpointManifest, expected_proposal_id: Option<&str>) -> Result<(), PatchError> {
    if manifest.version == 0 {
        return Err(PatchError::Rollback("checkpoint manifest version is required".into()));
    }
    if manifest.checkpoint_id.trim().is_empty() {
        return Err(PatchError::Rollback("checkpoint manifest checkpoint_id is required".into()));
    }
    if let Some(expected) = expected_proposal_id {
        if manifest.proposal_id != expected {
            return Err(PatchError::Rollback(format!("checkpoint manifest proposal_id mismatch: expected {}, got {}", expected, manifest.proposal_id)));
        }
    }
    let manifest_repo = PathBuf::from(&manifest.repo_root).canonicalize()?;
    let current_repo = repo_root.canonicalize()?;
    if manifest_repo != current_repo {
        return Err(PatchError::Rollback("checkpoint manifest repo_root does not match requested repo".into()));
    }
    let guard = RepoGuard::new(repo_root, FilePolicy::default())?;
    for entry in &manifest.files {
        guard.resolve_for_write_path(&entry.path)?;
        if entry.existed_before {
            let rel_backup = entry.backup_path.as_ref().ok_or_else(|| PatchError::Rollback(format!("missing backup path for {}", entry.path)))?;
            if backup_path_escapes(rel_backup) {
                return Err(PatchError::Rollback(format!("backup path escapes checkpoint directory for {}", entry.path)));
            }
            let backup_abs = canonical_checkpoint.join(rel_backup);
            let backup_canonical = backup_abs.canonicalize()?;
            if !backup_canonical.starts_with(canonical_checkpoint) {
                return Err(PatchError::Rollback(format!("backup path is outside checkpoint directory for {}", entry.path)));
            }
        } else if entry.backup_path.is_some() {
            return Err(PatchError::Rollback(format!("created file {} must not have a backup path", entry.path)));
        }
    }
    Ok(())
}

fn backup_path_escapes(path: &str) -> bool {
    let p = Path::new(path);
    p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
}

fn checkpoint_id_from_dir(checkpoint_dir: &Path) -> String {
    checkpoint_dir.file_name().and_then(|s| s.to_str()).unwrap_or("checkpoint").to_string()
}

fn sanitize_id(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>()
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    secs.to_string()
}

pub fn parse_diff_header(diff: &str) -> Result<DiffHeader, PatchError> {
    let mut old_path = None;
    let mut new_path = None;
    let mut has_old_marker = false;
    let mut has_new_marker = false;
    let mut hunk_count = 0usize;
    let mut has_diff_git = false;

    for line in diff.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("binary files ") || lower.starts_with("gitattributes") || lower.starts_with("git binary patch") {
            return Err(PatchError::Rejected("binary patches are not supported".into()));
        }
        if lower.starts_with("rename from ") || lower.starts_with("rename to ") {
            return Err(PatchError::Rejected("renames are not supported".into()));
        }
        if lower.starts_with("new file mode") {
            return Err(PatchError::Rejected("file mode changes are not supported; create files with /dev/null markers only".into()));
        }
        if lower.starts_with("deleted file mode") {
            return Err(PatchError::Rejected("file deletion patches are not supported".into()));
        }
        if lower.starts_with("old mode") || lower.starts_with("new mode") {
            return Err(PatchError::Rejected("file mode changes are not supported".into()));
        }
        if let Some(rest) = line.strip_prefix("diff --git ") {
            has_diff_git = true;
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                old_path = Some(strip_diff_prefix(parts[0]));
                new_path = Some(strip_diff_prefix(parts[1]));
            }
        } else if let Some(rest) = line.strip_prefix("--- ") {
            has_old_marker = true;
            let path = rest.split_whitespace().next().unwrap_or(rest);
            old_path = Some(strip_diff_prefix(path));
        } else if let Some(rest) = line.strip_prefix("+++ ") {
            has_new_marker = true;
            let path = rest.split_whitespace().next().unwrap_or(rest);
            new_path = Some(strip_diff_prefix(path));
        } else if line.starts_with("@@") {
            hunk_count += 1;
        }
    }
    if !has_diff_git {
        return Err(PatchError::Rejected("unified diff must include diff --git header".into()));
    }
    let old_path = old_path.ok_or_else(|| PatchError::Rejected("unified diff must include old path".into()))?;
    let new_path = new_path.ok_or_else(|| PatchError::Rejected("unified diff must include new path".into()))?;
    if !has_old_marker || !has_new_marker {
        return Err(PatchError::Rejected("unified diff must include --- and +++ file markers".into()));
    }
    if hunk_count == 0 {
        return Err(PatchError::Rejected("unified diff must contain at least one @@ hunk".into()));
    }
    Ok(DiffHeader { old_path, new_path, has_old_marker, has_new_marker, hunk_count })
}

fn validate_diff_paths_match(path: &str, header: &DiffHeader) -> Result<(), PatchError> {
    let expected = normalize_patch_path(path);
    let old = normalize_patch_path(&header.old_path);
    let new = normalize_patch_path(&header.new_path);
    let old_ok = old == expected || old == "/dev/null";
    let new_ok = new == expected || new == "/dev/null";
    if !old_ok || !new_ok {
        return Err(PatchError::Rejected(format!("diff header paths do not match PatchFile.path: expected {}, got old={} new={}", expected, old, new)));
    }
    if old == "/dev/null" && new == "/dev/null" {
        return Err(PatchError::Rejected("diff cannot have both old and new path as /dev/null".into()));
    }
    Ok(())
}

fn strip_diff_prefix(path: &str) -> String {
    let path = path.trim().trim_matches('"');
    if path == "/dev/null" { return path.to_string(); }
    path.strip_prefix("a/").or_else(|| path.strip_prefix("b/")).unwrap_or(path).to_string()
}

fn normalize_patch_path(path: &str) -> String {
    strip_diff_prefix(path).replace('\\', "/").trim_start_matches("./").to_string()
}

fn classify_patch(path: &str, diff: &str) -> PatchRisk {
    let p = path.to_ascii_lowercase();
    let d = diff.to_ascii_lowercase();
    if p.contains(".env") || p.contains(".ssh") || p.contains(".aws") || p.contains("id_rsa") || p.contains("secret") || p.contains("credential") || p.starts_with(".git/") {
        return PatchRisk::Blocked;
    }
    if p.ends_with("package.json") || p.ends_with("package-lock.json") || p.ends_with("pnpm-lock.yaml") || p.ends_with("yarn.lock") || p.ends_with("dockerfile") || p.contains("/.github/") || d.contains("postinstall") || d.contains("preinstall") {
        return PatchRisk::High;
    }
    if d.lines().filter(|l| l.starts_with('-')).count() > 20 || d.lines().filter(|l| l.starts_with('+')).count() > 20 {
        return PatchRisk::Medium;
    }
    PatchRisk::Low
}

pub fn apply_unified_diff_to_text(original: &str, diff: &str) -> Result<String, PatchError> {
    let header = parse_diff_header(diff)?;
    if header.hunk_count == 0 {
        return Err(PatchError::Apply("patch has no hunks".into()));
    }

    let mut lines: Vec<String> = if original.is_empty() {
        Vec::new()
    } else {
        original.split_inclusive('\n').map(|s| s.to_string()).collect()
    };

    let mut diff_lines = diff.lines().peekable();
    let mut applied_hunks = 0usize;
    let mut line_offset: isize = 0;

    while let Some(line) = diff_lines.next() {
        if !line.starts_with("@@") {
            continue;
        }
        applied_hunks += 1;
        let (old_start, old_count, _new_start, new_count) = parse_hunk_header(line)?;
        let adjusted_cursor = (old_start as isize - 1 + line_offset).max(0) as usize;
        if adjusted_cursor > lines.len() {
            return Err(PatchError::Apply(format!("hunk start {} is out of range after offset {}", old_start, line_offset)));
        }

        let mut replacement: Vec<String> = Vec::new();
        let mut consumed_original = 0usize;
        let mut emitted_new = 0usize;
        let mut hunk_old_seen = 0usize;
        let mut hunk_new_seen = 0usize;

        while let Some(next) = diff_lines.peek().cloned() {
            if next.starts_with("@@") || next.starts_with("diff --git") {
                break;
            }
            let next = diff_lines.next().unwrap();
            if next.starts_with("index ") {
                continue;
            }
            if next == r"\ No newline at end of file" {
                continue;
            }
            if next.starts_with("--- ") || next.starts_with("+++ ") {
                return Err(PatchError::Apply(format!("file marker appeared inside hunk: {}", next)));
            }
            if next.is_empty() {
                return Err(PatchError::Apply("empty diff line without prefix is unsupported".into()));
            }
            let (tag, body) = next.split_at(1);
            match tag {
                " " => {
                    let existing = lines.get(adjusted_cursor + consumed_original).ok_or_else(|| PatchError::Apply(format!("context line out of range: {}", body)))?;
                    if trim_line_ending(existing) != body {
                        return Err(PatchError::Apply(format!("context mismatch: expected {:?}, got {:?}", body, trim_line_ending(existing))));
                    }
                    replacement.push(existing.clone());
                    consumed_original += 1;
                    emitted_new += 1;
                    hunk_old_seen += 1;
                    hunk_new_seen += 1;
                }
                "-" => {
                    let existing = lines.get(adjusted_cursor + consumed_original).ok_or_else(|| PatchError::Apply(format!("removal line out of range: {}", body)))?;
                    if trim_line_ending(existing) != body {
                        return Err(PatchError::Apply(format!("removal mismatch: expected {:?}, got {:?}", body, trim_line_ending(existing))));
                    }
                    consumed_original += 1;
                    hunk_old_seen += 1;
                }
                "+" => {
                    replacement.push(format!("{}\n", body));
                    emitted_new += 1;
                    hunk_new_seen += 1;
                }
                _ => return Err(PatchError::Apply(format!("unsupported diff line prefix in hunk: {}", next))),
            }
        }

        if hunk_old_seen != old_count {
            return Err(PatchError::Apply(format!("hunk old line count mismatch: header={} seen={}", old_count, hunk_old_seen)));
        }
        if hunk_new_seen != new_count {
            return Err(PatchError::Apply(format!("hunk new line count mismatch: header={} seen={}", new_count, hunk_new_seen)));
        }
        lines.splice(adjusted_cursor..adjusted_cursor + consumed_original, replacement);
        line_offset += emitted_new as isize - consumed_original as isize;
    }

    if applied_hunks == 0 {
        return Err(PatchError::Apply("patch has no applyable hunks".into()));
    }
    Ok(lines.concat())
}

fn trim_line_ending(s: &str) -> &str {
    s.trim_end_matches(['\r', '\n'])
}

fn parse_hunk_header(line: &str) -> Result<(usize, usize, usize, usize), PatchError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 || parts[0] != "@@" {
        return Err(PatchError::Apply(format!("invalid hunk header: {}", line)));
    }
    let old = parse_range(parts[1].trim_start_matches('-'))?;
    let new = parse_range(parts[2].trim_start_matches('+'))?;
    Ok((old.0, old.1, new.0, new.1))
}

fn parse_range(input: &str) -> Result<(usize, usize), PatchError> {
    let mut parts = input.split(',');
    let start = parts.next().ok_or_else(|| PatchError::Apply(format!("invalid range: {}", input)))?.parse::<usize>().map_err(|_| PatchError::Apply(format!("invalid range start: {}", input)))?;
    let count = parts.next().unwrap_or("1").parse::<usize>().map_err(|_| PatchError::Apply(format!("invalid range count: {}", input)))?;
    Ok((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn file(path: &str, before_sha256: &str, diff: &str) -> PatchFile {
        PatchFile { id: "f1".into(), path: path.into(), before_sha256: before_sha256.into(), unified_diff: diff.into() }
    }

    fn valid_diff(path: &str, old: &str, new: &str) -> String {
        format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-{old}\n+{new}\n")
    }

    fn temp_repo() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("synthesize-patch-engine-test-{}-{}", std::process::id(), nanos));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn rejects_no_hunk_diff() {
        let diff = "diff --git a/src/a.ts b/src/a.ts\n--- a/src/a.ts\n+++ b/src/a.ts\n";
        let result = validate_patch_file_shape(&file("src/a.ts", "abc", diff));
        assert!(matches!(result, Err(PatchError::Rejected(_))));
    }

    #[test]
    fn rejects_diff_path_mismatch() {
        let diff = "diff --git a/package.json b/package.json\n--- a/package.json\n+++ b/package.json\n@@ -1 +1 @@\n-a\n+b\n";
        let result = validate_patch_file_shape(&file("src/a.ts", "abc", diff));
        assert!(matches!(result, Err(PatchError::Rejected(_))));
    }

    #[test]
    fn applies_simple_diff() {
        let original = "a\n";
        let diff = valid_diff("src/a.ts", "a", "b");
        let updated = apply_unified_diff_to_text(original, &diff).unwrap();
        assert_eq!(updated, "b\n");
    }

    #[test]
    fn rejects_no_op_patch() {
        let root = temp_repo();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.ts"), "a\n").unwrap();
        let before = sha256_file(&root.join("src/a.ts")).unwrap();
        let diff = valid_diff("src/a.ts", "a", "a");
        let proposal = PatchProposal { id: "p1".into(), base_commit: None, current_commit: None, files: vec![file("src/a.ts", &before, &diff)] };
        let result = apply_patch_proposal_transactional(&root, &proposal, None);
        assert!(result.is_err());
    }

    #[test]
    fn rollback_deletes_created_files() {
        let root = temp_repo();
        let empty = sha256_text("");
        let diff = "diff --git a/src/new.ts b/src/new.ts\n--- /dev/null\n+++ b/src/new.ts\n@@ -0,0 +1 @@\n+created\n";
        let proposal = PatchProposal { id: "p-create".into(), base_commit: None, current_commit: None, files: vec![file("src/new.ts", &empty, diff)] };
        let applied = apply_patch_proposal_transactional(&root, &proposal, None).unwrap();
        assert!(root.join("src/new.ts").exists());
        let rolled = rollback_checkpoint(&root, &applied.checkpoint_dir).unwrap();
        assert!(rolled.deleted_paths.contains(&"src/new.ts".to_string()));
        assert!(!root.join("src/new.ts").exists());
    }

    #[test]
    fn rollback_for_proposal_rejects_manifest_proposal_mismatch() {
        let root = temp_repo();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.ts"), "a\n").unwrap();
        let before = sha256_file(&root.join("src/a.ts")).unwrap();
        let diff = valid_diff("src/a.ts", "a", "b");
        let proposal = PatchProposal { id: "p-rollback-a".into(), base_commit: None, current_commit: None, files: vec![file("src/a.ts", &before, &diff)] };
        let applied = apply_patch_proposal_transactional(&root, &proposal, None).unwrap();
        let result = rollback_checkpoint_for_proposal(&root, &applied.checkpoint_id, "different-proposal");
        assert!(matches!(result, Err(PatchError::Rollback(_))));
    }

    #[test]
    fn checkpoint_manifest_rejects_backup_path_escape() {
        let root = temp_repo();
        let checkpoint = root.join(".synthesize").join("checkpoints").join("cp1");
        fs::create_dir_all(checkpoint.join("backups")).unwrap();
        fs::write(checkpoint.join("backups").join("good.bak"), "a\n").unwrap();
        let manifest = CheckpointManifest {
            version: 2,
            checkpoint_id: "cp1".into(),
            repo_root: root.to_string_lossy().to_string(),
            proposal_id: "p1".into(),
            created_at: "now".into(),
            files: vec![CheckpointFileEntry { path: "src/a.ts".into(), existed_before: true, backup_path: Some("../escape.bak".into()), before_sha256: Some(sha256_text("a\n")) }],
        };
        let result = validate_checkpoint_manifest(&root, &checkpoint, &manifest, Some("p1"));
        assert!(matches!(result, Err(PatchError::Rollback(_))));
    }

    #[test]
    fn rollback_for_proposal_restores_modified_files() {
        let root = temp_repo();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.ts"), "a\n").unwrap();
        let before = sha256_file(&root.join("src/a.ts")).unwrap();
        let diff = valid_diff("src/a.ts", "a", "b");
        let proposal = PatchProposal { id: "p-restore".into(), base_commit: None, current_commit: None, files: vec![file("src/a.ts", &before, &diff)] };
        let applied = apply_patch_proposal_transactional(&root, &proposal, None).unwrap();
        assert_eq!(fs::read_to_string(root.join("src/a.ts")).unwrap(), "b\n");
        let rolled = rollback_checkpoint_for_proposal(&root, &applied.checkpoint_id, "p-restore").unwrap();
        assert!(rolled.restored_paths.contains(&"src/a.ts".to_string()));
        assert_eq!(fs::read_to_string(root.join("src/a.ts")).unwrap(), "a\n");
    }

    #[test]
    fn applies_multi_hunk_with_offsets() {
        let original = "one
two
three
four
five
";
        let diff = "diff --git a/src/a.ts b/src/a.ts
--- a/src/a.ts
+++ b/src/a.ts
@@ -1,3 +1,4 @@
 one
+inserted
 two
 three
@@ -5,1 +6,1 @@
-five
+FIVE
";
        let updated = apply_unified_diff_to_text(original, diff).unwrap();
        assert_eq!(updated, "one
inserted
two
three
four
FIVE
");
    }

    #[test]
    fn rejects_context_mismatch() {
        let original = "one
two
";
        let diff = "diff --git a/src/a.ts b/src/a.ts
--- a/src/a.ts
+++ b/src/a.ts
@@ -1,2 +1,2 @@
 one
-three
+TWO
";
        let result = apply_unified_diff_to_text(original, diff);
        assert!(matches!(result, Err(PatchError::Apply(_))));
    }

    #[test]
    fn rejects_malformed_hunk_counts() {
        let original = "one
two
";
        let diff = "diff --git a/src/a.ts b/src/a.ts
--- a/src/a.ts
+++ b/src/a.ts
@@ -1,2 +1,2 @@
 one
-two
";
        let result = apply_unified_diff_to_text(original, diff);
        assert!(matches!(result, Err(PatchError::Apply(_))));
    }

    #[test]
    fn applies_nested_file_creation() {
        let root = temp_repo();
        let empty = sha256_text("");
        let diff = "diff --git a/src/new/nested/file.ts b/src/new/nested/file.ts
--- /dev/null
+++ b/src/new/nested/file.ts
@@ -0,0 +1,2 @@
+export const answer = 42;
+
";
        let proposal = PatchProposal { id: "p-create-nested".into(), base_commit: None, current_commit: None, files: vec![file("src/new/nested/file.ts", &empty, diff)] };
        let applied = apply_patch_proposal_transactional(&root, &proposal, None).unwrap();
        assert!(root.join("src/new/nested/file.ts").exists());
        let rolled = rollback_checkpoint_for_proposal(&root, &applied.checkpoint_id, "p-create-nested").unwrap();
        assert!(rolled.deleted_paths.contains(&"src/new/nested/file.ts".to_string()));
        assert!(!root.join("src/new/nested/file.ts").exists());
    }

    #[test]
    fn rejects_binary_patch() {
        let diff = "diff --git a/src/a.bin b/src/a.bin
Binary files a/src/a.bin and b/src/a.bin differ
";
        let result = validate_patch_file_shape(&file("src/a.bin", "abc", diff));
        assert!(matches!(result, Err(PatchError::Rejected(_))));
    }

    #[test]
    fn rejects_rename_delete_and_mode_change() {
        let rename = "diff --git a/src/a.ts b/src/b.ts
rename from src/a.ts
rename to src/b.ts
";
        assert!(matches!(validate_patch_file_shape(&file("src/a.ts", "abc", rename)), Err(PatchError::Rejected(_))));
        let delete = "diff --git a/src/a.ts b/src/a.ts
deleted file mode 100644
--- a/src/a.ts
+++ /dev/null
@@ -1 +0,0 @@
-a
";
        assert!(matches!(validate_patch_file_shape(&file("src/a.ts", "abc", delete)), Err(PatchError::Rejected(_))));
        let mode = "diff --git a/src/a.ts b/src/a.ts
old mode 100644
new mode 100755
--- a/src/a.ts
+++ b/src/a.ts
@@ -1 +1 @@
-a
+b
";
        assert!(matches!(validate_patch_file_shape(&file("src/a.ts", "abc", mode)), Err(PatchError::Rejected(_))));
    }

}
