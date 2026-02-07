use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tracing::debug;

/// RAII handle for a PID file. Removes the file on drop.
#[derive(Debug)]
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Acquire a PID file for single-instance enforcement.
    ///
    /// Derives the PID path from the socket path (e.g. `foo.sock` → `foo.pid`).
    /// If a PID file already exists and the referenced process is alive, returns
    /// an error. If the process is dead, removes the stale PID file.
    pub fn acquire(socket_path: &Path) -> Result<Self> {
        let pid_path = derive_pid_path(socket_path);
        let current_pid = std::process::id();

        // Try atomic create first — if it succeeds, no other daemon is running.
        match write_pid_atomic(&pid_path, current_pid) {
            Ok(()) => {
                debug!(pid = current_pid, path = %pid_path.display(), "PID file acquired");
                Ok(Self { path: pid_path })
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // File already exists — check if the process is alive.
                let contents =
                    fs::read_to_string(&pid_path).context("failed to read existing PID file")?;

                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if is_process_alive(pid) {
                        bail!(
                            "another daemon is already running (PID {} from {})",
                            pid,
                            pid_path.display()
                        );
                    }
                    debug!(pid, "removing stale PID file (process is dead)");
                }

                // Stale or corrupt PID file — remove and recreate atomically.
                if let Err(e) = fs::remove_file(&pid_path) {
                    if e.kind() != io::ErrorKind::NotFound {
                        return Err(e).with_context(|| {
                            format!("failed to remove stale PID file: {}", pid_path.display())
                        });
                    }
                }
                write_pid_atomic(&pid_path, current_pid)
                    .with_context(|| format!("failed to write PID file: {}", pid_path.display()))?;

                debug!(pid = current_pid, path = %pid_path.display(), "PID file acquired (stale replaced)");
                Ok(Self { path: pid_path })
            }
            Err(e) => {
                // Real I/O error (permission denied, missing parent dir, etc.)
                Err(e).with_context(|| format!("failed to create PID file: {}", pid_path.display()))
            }
        }
    }

    /// Return the path of this PID file.
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            if e.kind() != io::ErrorKind::NotFound {
                tracing::warn!("failed to remove PID file {}: {}", self.path.display(), e);
            }
        }
    }
}

/// Atomically create a PID file using `create_new(true)`.
///
/// Returns `Ok(())` if the file was created, or `Err` if it already exists.
fn write_pid_atomic(path: &Path, pid: u32) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    write!(file, "{pid}")?;
    Ok(())
}

/// Derive the PID file path from a socket path.
///
/// Replaces the extension: `foo.sock` → `foo.pid`.
/// If no extension, appends `.pid`.
pub fn derive_pid_path(socket_path: &Path) -> PathBuf {
    socket_path.with_extension("pid")
}

/// Check whether a process with the given PID is alive.
///
/// Uses `kill(pid, 0)` which checks existence without sending a signal.
/// On macOS, `EPERM` means the process exists but belongs to a different user.
pub fn is_process_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        // PID overflows pid_t (i32) — not a valid process
        return false;
    };
    // Safety: kill(pid, 0) is safe — it only checks if the process exists.
    let ret = unsafe { libc::kill(pid, 0) };
    if ret == 0 {
        return true;
    }
    // EPERM = process exists but we lack permission → it's alive
    // ESRCH = no such process → it's dead
    io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn derive_pid_path_from_sock() {
        let socket = Path::new("/tmp/autocomplete-rs.sock");
        assert_eq!(
            derive_pid_path(socket),
            Path::new("/tmp/autocomplete-rs.pid")
        );
    }

    #[test]
    fn derive_pid_path_no_extension() {
        let socket = Path::new("/tmp/my-daemon");
        assert_eq!(derive_pid_path(socket), Path::new("/tmp/my-daemon.pid"));
    }

    #[test]
    fn current_process_is_alive() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn dead_process_is_not_alive() {
        // PID 99999999 is extremely unlikely to exist
        assert!(!is_process_alive(99_999_999));
    }

    #[test]
    fn acquire_and_release() {
        let dir = std::env::temp_dir();
        let sock = dir.join(format!("test-pid-{}.sock", std::process::id()));
        let pid_path = derive_pid_path(&sock);

        // Ensure clean state
        if let Err(e) = fs::remove_file(&pid_path) {
            if e.kind() != io::ErrorKind::NotFound {
                panic!(
                    "failed to clean up test PID file {}: {}",
                    pid_path.display(),
                    e
                );
            }
        }

        {
            let pf = PidFile::acquire(&sock).unwrap();
            assert!(pf.path().exists());

            let contents = fs::read_to_string(pf.path()).unwrap();
            assert_eq!(contents, std::process::id().to_string());
        }
        // Drop should have cleaned up
        assert!(!pid_path.exists());
    }

    #[test]
    fn double_acquire_fails_for_live_process() {
        let dir = std::env::temp_dir();
        let sock = dir.join(format!("test-double-{}.sock", std::process::id()));
        let pid_path = derive_pid_path(&sock);
        if let Err(e) = fs::remove_file(&pid_path) {
            if e.kind() != io::ErrorKind::NotFound {
                panic!(
                    "failed to clean up test PID file {}: {}",
                    pid_path.display(),
                    e
                );
            }
        }

        let _pf = PidFile::acquire(&sock).unwrap();

        // Second acquire should fail because our process is alive
        let result = PidFile::acquire(&sock);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("another daemon is already running")
        );
    }

    #[test]
    fn stale_pid_file_cleaned_up() {
        let dir = std::env::temp_dir();
        let sock = dir.join(format!("test-stale-{}.sock", std::process::id()));
        let pid_path = derive_pid_path(&sock);

        // Write a stale PID file (PID that doesn't exist)
        fs::write(&pid_path, "99999999").unwrap();

        // Should succeed because the stale process is dead
        let pf = PidFile::acquire(&sock).unwrap();
        let contents = fs::read_to_string(pf.path()).unwrap();
        assert_eq!(contents, std::process::id().to_string());
    }
}
