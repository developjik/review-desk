use crate::domain::{AnalysisRun, Result, ReviewDeskError, ReviewDraft};
use crate::storage::LocalStore;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrWorkspaceKey {
    owner: String,
    repo: String,
    number: u64,
}

impl PrWorkspaceKey {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, number: u64) -> Result<Self> {
        let owner = owner.into();
        let repo = repo.into();
        validate_segment("owner", &owner)?;
        validate_segment("repo", &repo)?;
        Ok(Self {
            owner,
            repo,
            number,
        })
    }

    fn base_relative(&self) -> String {
        format!("workspaces/{}/{}/{}", self.owner, self.repo, self.number)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    local: LocalStore,
}

impl WorkspaceStore {
    pub fn init(project_root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            local: LocalStore::init(project_root)?,
        })
    }

    pub fn save_run(&self, key: &PrWorkspaceKey, run: &AnalysisRun) -> Result<PathBuf> {
        validate_segment("run_id", &run.run_id)?;
        validate_pr_identity(key, &run.owner, &run.repo, run.number)?;
        self.ensure_workspace_dirs(key)?;
        let relative = format!("{}/runs/{}.json", key.base_relative(), run.run_id);
        let path = self.local.root().join(&relative);
        let data = serde_json::to_vec_pretty(run)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&data)?;
        set_owner_only_file(&path)?;
        Ok(path)
    }

    pub fn list_runs(&self, key: &PrWorkspaceKey) -> Result<Vec<AnalysisRun>> {
        let runs_dir = self.local.root().join(key.base_relative()).join("runs");
        if !runs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut paths = Vec::new();
        for entry in std::fs::read_dir(runs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        paths.sort();

        let mut runs = paths
            .into_iter()
            .map(|path| {
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| ReviewDeskError::InvalidPath(path.display().to_string()))?;
                let run: AnalysisRun =
                    self.local
                        .load_json(&format!("{}/runs/{}", key.base_relative(), file_name))?;
                validate_pr_identity(key, &run.owner, &run.repo, run.number)?;
                Ok(run)
            })
            .collect::<Result<Vec<_>>>()?;
        runs.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.run_id.cmp(&right.run_id))
        });
        Ok(runs)
    }

    pub fn save_draft(&self, key: &PrWorkspaceKey, draft: &ReviewDraft) -> Result<PathBuf> {
        validate_segment("draft_id", &draft.draft_id)?;
        validate_pr_identity(key, &draft.owner, &draft.repo, draft.number)?;
        self.ensure_workspace_dirs(key)?;
        self.local.save_json(
            &format!("{}/drafts/{}.json", key.base_relative(), draft.draft_id),
            draft,
        )
    }

    pub fn read_draft(&self, key: &PrWorkspaceKey, draft_id: &str) -> Result<ReviewDraft> {
        validate_segment("draft_id", draft_id)?;
        let draft: ReviewDraft =
            self.local
                .load_json(&format!("{}/drafts/{}.json", key.base_relative(), draft_id))?;
        validate_pr_identity(key, &draft.owner, &draft.repo, draft.number)?;
        Ok(draft)
    }

    pub fn mark_active_draft(&self, key: &PrWorkspaceKey, draft_id: &str) -> Result<PathBuf> {
        validate_segment("draft_id", draft_id)?;
        self.read_draft(key, draft_id)?;
        self.ensure_workspace_dirs(key)?;
        let draft_id = draft_id.to_string();
        self.local.save_json(
            &format!("{}/drafts/active.json", key.base_relative()),
            &draft_id,
        )
    }

    pub fn read_active_draft_id(&self, key: &PrWorkspaceKey) -> Result<Option<String>> {
        let path = self
            .local
            .root()
            .join(key.base_relative())
            .join("drafts")
            .join("active.json");
        if !path.exists() {
            return Ok(None);
        }

        let draft_id: String = self
            .local
            .load_json(&format!("{}/drafts/active.json", key.base_relative()))?;
        validate_segment("draft_id", &draft_id)?;
        Ok(Some(draft_id))
    }

    fn ensure_workspace_dirs(&self, key: &PrWorkspaceKey) -> Result<()> {
        let workspace_root = self.local.root().join("workspaces");
        let owner_root = workspace_root.join(&key.owner);
        let repo_root = owner_root.join(&key.repo);
        let pr_root = repo_root.join(key.number.to_string());
        let dirs = [
            workspace_root,
            owner_root,
            repo_root,
            pr_root.clone(),
            pr_root.join("snapshots"),
            pr_root.join("runs"),
            pr_root.join("drafts"),
            pr_root.join("publish_attempts"),
        ];

        for dir in dirs {
            std::fs::create_dir_all(&dir)?;
            set_owner_only_dir(&dir)?;
        }

        Ok(())
    }
}

fn validate_pr_identity(key: &PrWorkspaceKey, owner: &str, repo: &str, number: u64) -> Result<()> {
    if key.owner != owner || key.repo != repo || key.number != number {
        return Err(ReviewDeskError::InvalidPath(format!(
            "artifact PR identity does not match workspace key: {owner}/{repo}#{number}"
        )));
    }
    Ok(())
}

fn validate_segment(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value == ".."
        || value.starts_with('.')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ReviewDeskError::InvalidPath(format!(
            "invalid {label} segment: {value}"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_dir(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_file(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_file(_path: &Path) -> Result<()> {
    Ok(())
}
