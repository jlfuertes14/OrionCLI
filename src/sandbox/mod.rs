use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use crate::config::Settings;

/// Resolves and validates a path against allowed directories in the settings.
/// If `allowed_dirs` is empty, it returns the resolved path (no restrictions, standard directory tracking).
pub fn validate_path(path_str: &str, settings: &Settings) -> Result<PathBuf> {
    let raw_path = Path::new(path_str);
    
    // Resolve absolute path
    let resolved = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };

    // Canonicalize if the path exists, otherwise clean it by removing relative components
    let canonical = match resolved.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Fallback to simple normalization for non-existent paths (e.g. creating a new file)
            normalize_path(&resolved)
        }
    };

    if settings.allowed_dirs.is_empty() {
        return Ok(canonical);
    }

    for allowed in &settings.allowed_dirs {
        let allowed_path = Path::new(allowed).canonicalize()?;
        if canonical.starts_with(&allowed_path) {
            return Ok(canonical);
        }
    }

    Err(anyhow!(
        "Access denied: '{:?}' is outside allowed directories. Allowed directories: {:?}",
        canonical,
        settings.allowed_dirs
    ))
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
