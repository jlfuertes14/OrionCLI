use crate::config::Settings;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Resolves and validates a path against the workspace and configured allowed directories.
/// The active workspace is always allowed. Any path outside the workspace requires an
/// explicit entry in `allowed_dirs`.
pub fn validate_path(path_str: &str, settings: &Settings) -> Result<PathBuf> {
    let raw_path = Path::new(path_str);

    // Resolve absolute path
    let resolved = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        settings.workspace_dir.join(raw_path)
    };

    // Canonicalize if the path exists, otherwise clean it by removing relative components
    let canonical = strip_windows_verbatim_prefix(&match resolved.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Fallback to simple normalization for non-existent paths (e.g. creating a new file)
            normalize_path(&resolved)
        }
    });

    let workspace = strip_windows_verbatim_prefix(
        &settings
            .workspace_dir
            .canonicalize()
            .unwrap_or_else(|_| normalize_path(&settings.workspace_dir)),
    );
    if canonical.starts_with(&workspace) {
        return Ok(canonical);
    }

    for allowed in &settings.allowed_dirs {
        let allowed_raw = Path::new(allowed);
        let allowed_path = strip_windows_verbatim_prefix(
            &allowed_raw
                .canonicalize()
                .unwrap_or_else(|_| normalize_path(allowed_raw)),
        );
        if canonical.starts_with(&allowed_path) {
            return Ok(canonical);
        }
    }

    Err(anyhow!(
        "Access denied: '{:?}' is outside the workspace or allowed directories. Workspace: {:?}. Allowed directories: {:?}",
        canonical,
        workspace,
        settings.allowed_dirs
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_parent_escape_when_no_allowed_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        settings.workspace_dir = dir.path().join("workspace");
        std::fs::create_dir_all(&settings.workspace_dir).unwrap();
        settings.allowed_dirs.clear();

        let err = validate_path("../outside.txt", &settings).unwrap_err();
        assert!(err.to_string().contains("Access denied"));
    }

    #[test]
    fn allows_workspace_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::default();
        settings.workspace_dir = dir.path().join("workspace");
        std::fs::create_dir_all(&settings.workspace_dir).unwrap();
        settings.allowed_dirs.clear();

        let path = validate_path("inside.txt", &settings).unwrap();
        assert!(path.starts_with(&settings.workspace_dir));
    }
}

/// Helper to normalize paths without relying on filesystem canonicalize (for non-existent paths)
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
            Component::CurDir => {}
            _ => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

#[cfg(windows)]
fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    let value = path.to_string_lossy();
    if let Some(stripped) = value.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{}", stripped))
    } else if let Some(stripped) = value.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    path.to_path_buf()
}
