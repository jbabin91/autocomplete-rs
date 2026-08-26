use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
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

/// Whether `name` is a file the daily rolling appender produces.
///
/// Matches `autocomplete-rs.log.YYYY-MM-DD` exactly, so unrelated files that merely share
/// the prefix are never deleted.
fn is_rolling_log_name(name: &str) -> bool {
    let Some(date) = name.strip_prefix(&format!("{LOG_FILE_PREFIX}.")) else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
}

/// Base name the rolling appender writes; daily files append `.YYYY-MM-DD`.
pub const LOG_FILE_PREFIX: &str = "autocomplete-rs.log";

/// Return the default log directory: `~/.autocomplete-rs/logs/`.
pub fn default_log_dir() -> Result<PathBuf> {
    Ok(crate::paths::data_dir()?.join("logs"))
}

/// Return a custom log directory from env, or the default.
pub fn resolve_log_dir() -> Result<PathBuf> {
    match std::env::var_os("AUTOCOMPLETE_LOG_DIR") {
        Some(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
        _ => default_log_dir(),
    }
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

/// Create the log directory at mode 0700, along with any of our own ancestors.
///
/// Returns what was acted on so the caller can log it once a subscriber exists; this runs
/// before one does. A log directory outside our data directory is validated, not modified:
/// `AUTOCOMPLETE_LOG_DIR` can name anywhere, and chmodding it could revoke access to
/// unrelated files.
pub fn ensure_log_dir(path: &Path) -> Result<Vec<(PathBuf, crate::paths::DirAction)>> {
    crate::paths::ensure_private_dir(path)
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
        // The rolling appender writes `autocomplete-rs.log.YYYY-MM-DD`, whose extension is
        // the date — matching on `.log` skipped every file and deleted nothing. Match the
        // date shape too, so an adjacent `autocomplete-rs.log.backup` is left alone.
        let is_log_file = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(is_rolling_log_name);
        if !is_log_file {
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

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

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
    fn test_ensure_log_dir_reports_insecure_custom_dir_without_mutating_it() {
        // AUTOCOMPLETE_LOG_DIR can name any directory. Silently chmodding a path the user
        // chose could revoke access to unrelated files, so a custom dir is reported on.
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o755)).unwrap();

        let err = ensure_log_dir(&log_dir).expect_err("a custom 0755 log dir must be reported");
        assert!(
            err.to_string().contains("chmod 700"),
            "error should be actionable: {err}"
        );

        let perms = fs::metadata(&log_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            perms, 0o755,
            "custom dir must be left untouched, got {perms:o}"
        );
    }

    #[test]
    fn test_ensure_log_dir_tightens_the_shared_data_root() {
        // Regression: create_dir_all left the shared data directory at the umask default,
        // which then blocked the daemon socket from binding in that same directory.
        let tmp = tempfile::tempdir().unwrap();
        crate::paths::tests::with_home(Some(tmp.path().to_str().unwrap()), || {
            let root = tmp.path().join(".autocomplete-rs");
            fs::create_dir_all(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();

            let log_dir = default_log_dir().unwrap();
            // Pre-create the leaf too, so both levels exercise repair rather than the leaf
            // being created private and never needing it.
            fs::create_dir_all(&log_dir).unwrap();
            fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o755)).unwrap();

            ensure_log_dir(&log_dir).expect("the default layout is ours to repair");

            let root_perms = fs::metadata(&root).unwrap().permissions().mode() & 0o777;
            assert_eq!(root_perms, 0o700, "data root left at {root_perms:o}");
            let leaf_perms = fs::metadata(&log_dir).unwrap().permissions().mode() & 0o777;
            assert_eq!(leaf_perms, 0o700, "log dir left at {leaf_perms:o}");
        });
    }

    #[test]
    fn test_cleanup_old_logs() {
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path();

        // The appender writes `autocomplete-rs.log.YYYY-MM-DD`. Using any other shape here
        // is what let a filter that matched `extension() == "log"` pass while deleting
        // nothing in production.
        let log_file = log_dir.join(format!("{LOG_FILE_PREFIX}.2025-01-01"));
        let mut f = fs::File::create(&log_file).unwrap();
        writeln!(f, "recent log").unwrap();

        // cleanup_old_logs with 0 days should remove everything
        cleanup_old_logs(log_dir, 0).unwrap();
        assert!(
            !log_file.exists(),
            "a file named as the appender names it must be eligible for cleanup"
        );
    }

    #[test]
    fn test_cleanup_spares_files_that_merely_share_the_prefix() {
        // Widening the filter from `extension() == "log"` to a prefix match fixed
        // "deletes nothing" but would have deleted these.
        let tmp = tempfile::tempdir().unwrap();
        let keep = [
            format!("{LOG_FILE_PREFIX}.backup"),
            LOG_FILE_PREFIX.to_string(),
            "autocomplete-rs.logging-notes".to_string(),
            format!("{LOG_FILE_PREFIX}.2026-8-1"),
        ];
        for name in &keep {
            fs::write(tmp.path().join(name), b"keep me").unwrap();
        }
        let rolled = tmp.path().join(format!("{LOG_FILE_PREFIX}.2025-01-01"));
        fs::write(&rolled, b"delete me").unwrap();

        cleanup_old_logs(tmp.path(), 0).unwrap();

        for name in &keep {
            assert!(
                tmp.path().join(name).exists(),
                "{name} is not a rolled log file and must be left alone"
            );
        }
        assert!(!rolled.exists(), "a rolled log file must be eligible");
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
        let dir = default_log_dir().expect("HOME is set in the test environment");
        assert!(dir.to_string_lossy().contains("autocomplete-rs"));
        assert!(dir.to_string_lossy().contains("logs"));
    }
}
