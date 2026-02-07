use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, bail};
use tracing::warn;

use super::Mode;

/// Detect the logging mode from environment variables.
///
/// - `AUTOCOMPLETE_DEV=1` → Development
/// - `RUST_LOG` set → Troubleshooting
/// - Otherwise → Production
pub fn detect_mode() -> Mode {
    if std::env::var("AUTOCOMPLETE_DEV").as_deref() == Ok("1") {
        Mode::Development
    } else if std::env::var("RUST_LOG").is_ok() {
        Mode::Troubleshooting
    } else {
        Mode::Production
    }
}

/// Return the default log directory: `~/.autocomplete-rs/logs/`.
pub fn default_log_dir() -> PathBuf {
    dirs_or_home().join(".autocomplete-rs").join("logs")
}

/// Return a custom log directory from env, or the default.
pub fn resolve_log_dir() -> PathBuf {
    std::env::var("AUTOCOMPLETE_LOG_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(default_log_dir)
}

/// Return the retention days for a given mode.
pub fn retention_days(mode: &Mode) -> u32 {
    match mode {
        Mode::Production => 7,
        Mode::Development => 30,
        Mode::Troubleshooting => 90,
    }
}

/// Whether console output is enabled for a given mode.
pub fn console_enabled(mode: &Mode) -> bool {
    if std::env::var("AUTOCOMPLETE_CONSOLE").as_deref() == Ok("1") {
        return true;
    }
    matches!(mode, Mode::Development)
}

/// Create the log directory with 0700 permissions, or validate existing perms.
pub fn ensure_log_dir(path: &Path) -> Result<()> {
    if path.exists() {
        let metadata = fs::metadata(path).context("failed to read log directory metadata")?;
        if !metadata.is_dir() {
            bail!(
                "log directory path {} exists but is not a directory",
                path.display()
            );
        }
        let perms = metadata.permissions().mode() & 0o777;
        if perms & 0o077 != 0 {
            bail!(
                "log directory {} has insecure permissions {:o} (expected 0700)",
                path.display(),
                perms
            );
        }
        Ok(())
    } else {
        fs::create_dir_all(path).context("failed to create log directory")?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .context("failed to set log directory permissions")?;
        Ok(())
    }
}

/// Delete log files older than `retention_days` from the given directory.
pub fn cleanup_old_logs(dir: &Path, days: u32) -> Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(u64::from(days) * 86400))
        .context("time overflow computing retention cutoff")?;

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).context("failed to read log directory"),
    };

    for entry in entries {
        let entry = entry.context("failed to read directory entry")?;
        let path = entry.path();

        // Only clean up .log files
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                warn!("failed to stat log file {}: {}", path.display(), e);
                continue;
            }
        };

        let modified = metadata
            .modified()
            .context("failed to read modification time")?;

        if modified < cutoff
            && let Err(e) = fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!("failed to remove old log {}: {}", path.display(), e);
        }
    }

    Ok(())
}

/// Fallback home directory resolution.
fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mode_production() {
        // Clear relevant env vars
        // SAFETY: test-only, single-threaded access to these env vars
        unsafe {
            std::env::remove_var("AUTOCOMPLETE_DEV");
            std::env::remove_var("RUST_LOG");
        }
        assert!(matches!(detect_mode(), Mode::Production));
    }

    #[test]
    fn test_retention_days() {
        assert_eq!(retention_days(&Mode::Production), 7);
        assert_eq!(retention_days(&Mode::Development), 30);
        assert_eq!(retention_days(&Mode::Troubleshooting), 90);
    }

    #[test]
    fn test_ensure_log_dir_creates() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        assert!(!log_dir.exists());
        ensure_log_dir(&log_dir).unwrap();
        assert!(log_dir.exists());
        let perms = fs::metadata(&log_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(perms, 0o700);
    }

    #[test]
    fn test_ensure_log_dir_existing_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o700)).unwrap();
        ensure_log_dir(&log_dir).unwrap();
    }

    #[test]
    fn test_ensure_log_dir_insecure_perms() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o755)).unwrap();
        let result = ensure_log_dir(&log_dir);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("insecure permissions")
        );
    }

    #[test]
    fn test_cleanup_old_logs() {
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path();

        // Create a "new" log file
        let new_file = log_dir.join("autocomplete-rs.2025-01-01.log");
        let mut f = fs::File::create(&new_file).unwrap();
        writeln!(f, "recent log").unwrap();

        // cleanup_old_logs with 0 days should remove everything
        cleanup_old_logs(log_dir, 0).unwrap();
        assert!(!new_file.exists());
    }

    #[test]
    fn test_cleanup_ignores_non_log_files() {
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path();

        let txt_file = log_dir.join("notes.txt");
        let mut f = fs::File::create(&txt_file).unwrap();
        writeln!(f, "keep me").unwrap();

        cleanup_old_logs(log_dir, 0).unwrap();
        assert!(txt_file.exists());
    }

    #[test]
    fn test_default_log_dir_contains_autocomplete() {
        let dir = default_log_dir();
        assert!(dir.to_string_lossy().contains("autocomplete-rs"));
        assert!(dir.to_string_lossy().contains("logs"));
    }
}
