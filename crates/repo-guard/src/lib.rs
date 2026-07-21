use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoGuardError {
    #[error("path is outside repository")]
    OutsideRepo,
    #[error("path is denied by policy: {0}")]
    Denied(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct FilePolicy {
    pub denied_basenames: Vec<String>,
    pub denied_contains: Vec<String>,
    pub allow_hidden_files: bool,
}

impl Default for FilePolicy {
    fn default() -> Self {
        Self {
            denied_basenames: vec![
                ".env".into(),
                ".npmrc".into(),
                ".pypirc".into(),
                "id_rsa".into(),
                "id_ed25519".into(),
            ],
            denied_contains: vec![
                ".ssh".into(),
                ".aws".into(),
                ".gnupg".into(),
                "credentials".into(),
                "private_key".into(),
            ],
            allow_hidden_files: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoGuard {
    root: PathBuf,
    canonical_root: PathBuf,
    policy: FilePolicy,
}

impl RepoGuard {
    pub fn new(root: impl AsRef<Path>, policy: FilePolicy) -> Result<Self, RepoGuardError> {
        let root = root.as_ref().to_path_buf();
        let canonical_root = root.canonicalize()?;
        Ok(Self {
            root,
            canonical_root,
            policy,
        })
    }

    pub fn resolve_for_existing_path(
        &self,
        candidate: impl AsRef<Path>,
    ) -> Result<PathBuf, RepoGuardError> {
        let joined = self.root.join(candidate.as_ref());
        let canonical = joined.canonicalize()?;
        self.assert_inside(&canonical)?;
        self.assert_allowed(&canonical)?;
        Ok(canonical)
    }

    pub fn resolve_for_write_path(
        &self,
        candidate: impl AsRef<Path>,
    ) -> Result<PathBuf, RepoGuardError> {
        let candidate = candidate.as_ref();
        self.assert_relative_safe(candidate)?;

        let joined = self.root.join(candidate);
        if joined.exists() {
            let canonical = joined.canonicalize()?;
            self.assert_inside(&canonical)?;
            self.assert_allowed(&canonical)?;
            return Ok(canonical);
        }

        // New files may live inside new directories. Canonicalize the nearest existing
        // ancestor, prove that ancestor is inside the repo, then rebuild the remaining
        // path lexically underneath it. This avoids requiring the immediate parent to
        // exist while still rejecting traversal, absolute paths, and symlink escapes in
        // existing ancestors.
        let mut existing_ancestor = joined.as_path();
        let mut missing_components: Vec<PathBuf> = Vec::new();
        while !existing_ancestor.exists() {
            let name = existing_ancestor
                .file_name()
                .ok_or(RepoGuardError::OutsideRepo)?;
            missing_components.push(PathBuf::from(name));
            existing_ancestor = existing_ancestor
                .parent()
                .ok_or(RepoGuardError::OutsideRepo)?;
        }
        let canonical_ancestor = existing_ancestor.canonicalize()?;
        self.assert_inside(&canonical_ancestor)?;
        self.assert_allowed(&canonical_ancestor)?;

        let mut final_path = canonical_ancestor;
        for component in missing_components.iter().rev() {
            final_path.push(component);
        }
        self.assert_inside_lexical(&final_path)?;
        self.assert_allowed(&final_path)?;
        Ok(final_path)
    }

    fn assert_relative_safe(&self, path: &Path) -> Result<(), RepoGuardError> {
        for component in path.components() {
            match component {
                std::path::Component::Normal(_) => {}
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_) => {
                    return Err(RepoGuardError::OutsideRepo);
                }
            }
        }
        Ok(())
    }

    fn assert_inside_lexical(&self, path: &Path) -> Result<(), RepoGuardError> {
        if path == self.canonical_root || path.starts_with(&self.canonical_root) {
            Ok(())
        } else {
            Err(RepoGuardError::OutsideRepo)
        }
    }

    fn assert_inside(&self, canonical: &Path) -> Result<(), RepoGuardError> {
        if canonical == self.canonical_root || canonical.starts_with(&self.canonical_root) {
            Ok(())
        } else {
            Err(RepoGuardError::OutsideRepo)
        }
    }

    fn assert_allowed(&self, path: &Path) -> Result<(), RepoGuardError> {
        let s = path.to_string_lossy().to_lowercase();
        for deny in &self.policy.denied_contains {
            if s.contains(&deny.to_lowercase()) {
                return Err(RepoGuardError::Denied(deny.clone()));
            }
        }
        let policy_path = path.strip_prefix(&self.canonical_root).unwrap_or(path);
        for component in policy_path.components() {
            let Some(name) = component.as_os_str().to_str() else {
                continue;
            };
            for deny in &self.policy.denied_basenames {
                if name.eq_ignore_ascii_case(deny) {
                    return Err(RepoGuardError::Denied(deny.clone()));
                }
            }
            if !self.policy.allow_hidden_files && name.starts_with('.') && name != ".gitignore" {
                return Err(RepoGuardError::Denied("hidden file".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn denies_parent_escape() {
        let tmp = tempfile_like("synthesize_repo_guard_escape");
        fs::create_dir_all(&tmp).unwrap();
        let guard = RepoGuard::new(&tmp, FilePolicy::default()).unwrap();
        let result = guard.resolve_for_existing_path("../");
        assert!(result.is_err());
    }

    #[test]
    fn allows_new_file_in_new_nested_directory() {
        let tmp = tempfile_like("synthesize_repo_guard_nested_new");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let guard = RepoGuard::new(&tmp, FilePolicy::default()).unwrap();
        let resolved = guard
            .resolve_for_write_path("src/generated/deep/new_file.ts")
            .unwrap();
        assert!(resolved.ends_with("src/generated/deep/new_file.ts"));
        assert!(resolved.starts_with(tmp.canonicalize().unwrap()));
    }

    #[test]
    fn denies_new_file_with_parent_traversal() {
        let tmp = tempfile_like("synthesize_repo_guard_nested_escape");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let guard = RepoGuard::new(&tmp, FilePolicy::default()).unwrap();
        let result = guard.resolve_for_write_path("src/../../outside.ts");
        assert!(result.is_err());
    }

    #[test]
    fn denies_hidden_directory_for_new_file() {
        let tmp = tempfile_like("synthesize_repo_guard_hidden_dir");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let guard = RepoGuard::new(&tmp, FilePolicy::default()).unwrap();
        let result = guard.resolve_for_write_path("src/.secrets/new.ts");
        assert!(result.is_err());
    }

    fn tempfile_like(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{}_{}", name, std::process::id()))
    }
}
