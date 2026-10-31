//! Pure host/storage policy, testable without opening a WebView or operational path.
use crate::Result;
use std::path::{Path, PathBuf};

pub fn validate_host(debug: bool, identifier: &str, window: &str) -> Result<()> {
    if debug && identifier == "dev.ygnio.lucille.phase1" && window == "main" {
        Ok(())
    } else {
        Err("Lucille phase 1 requires an isolated debug application".into())
    }
}

pub fn isolated_database_path(path: &Path, protected: &[PathBuf]) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err("Isolated root must be absolute".into());
    }
    let root = path
        .canonicalize()
        .map_err(|_| "Isolated root unavailable")?;
    for protected in protected {
        if let Ok(protected) = protected.canonicalize() {
            if root.starts_with(protected) {
                return Err("Operational storage is forbidden".into());
            }
        }
    }
    if std::fs::read_to_string(root.join(".lucille-isolated"))
        .ok()
        .as_deref()
        != Some("synthetic-data-only\n")
    {
        return Err("Isolated synthetic-data marker required".into());
    }
    for name in [
        "lucille-phase1.sqlite3",
        "lucille-phase1.sqlite3-wal",
        "lucille-phase1.sqlite3-shm",
    ] {
        let path = root.join(name);
        if let Ok(metadata) = path.symlink_metadata() {
            if metadata.file_type().is_symlink()
                || path
                    .canonicalize()
                    .map_err(|_| "Invalid database path")?
                    .parent()
                    != Some(root.as_path())
            {
                return Err("Database escapes isolated root".into());
            }
        }
    }
    Ok(root.join("lucille-phase1.sqlite3"))
}
