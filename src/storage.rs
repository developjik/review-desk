use crate::domain::{Result, ReviewDeskError};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    pub fn init(project_root: impl AsRef<Path>) -> Result<Self> {
        let root = project_root.as_ref().join(".reviewdesk");
        std::fs::create_dir_all(&root)?;
        set_owner_only_dir(&root)?;

        for child in [
            "reviews",
            "drafts",
            "submissions",
            "state",
            "debug",
            "workspaces",
        ] {
            let path = root.join(child);
            std::fs::create_dir_all(&path)?;
            set_owner_only_dir(&path)?;
        }

        let ignore = root.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(&ignore, "*\n")?;
            set_owner_only_file(&ignore)?;
        }

        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn save_json<T: Serialize>(&self, relative: &str, value: &T) -> Result<PathBuf> {
        let path = self.safe_path(relative)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            set_owner_only_dir(parent)?;
        }
        let data = serde_json::to_vec_pretty(value)?;
        std::fs::write(&path, data)?;
        set_owner_only_file(&path)?;
        Ok(path)
    }

    pub fn load_json<T: DeserializeOwned>(&self, relative: &str) -> Result<T> {
        let path = self.safe_path(relative)?;
        let data = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&data)?)
    }

    pub fn save_text(&self, relative: &str, value: &str) -> Result<PathBuf> {
        let path = self.safe_path(relative)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            set_owner_only_dir(parent)?;
        }
        std::fs::write(&path, value)?;
        set_owner_only_file(&path)?;
        Ok(path)
    }

    pub fn load_text(&self, relative: &str) -> Result<String> {
        let path = self.safe_path(relative)?;
        Ok(std::fs::read_to_string(path)?)
    }

    fn safe_path(&self, relative: &str) -> Result<PathBuf> {
        let path = Path::new(relative);
        if path.is_absolute() || relative.contains("..") {
            return Err(ReviewDeskError::InvalidPath(relative.to_string()));
        }
        Ok(self.root.join(path))
    }
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
