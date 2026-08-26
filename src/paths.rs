use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Size of the `sockaddr_un::sun_path` buffer, including the trailing NUL.
///
/// 104 on the BSDs and macOS, 108 on Linux; the smaller bound is used everywhere so a path
/// that binds on one platform is not silently rejected on another. The longest usable path
/// is therefore one byte less — measured on macOS, 103 binds and 104 fails.
pub const MAX_SOCKET_PATH_LEN: usize = 104;

/// Resolve the user's home directory from `$HOME`.
///
/// A missing `$HOME` is an error rather than a fallback: every caller uses this to build a
/// path that must be private, and the obvious fallback (`/tmp`) is exactly the
/// world-writable location [`default_socket_path`] exists to avoid.
pub(crate) fn home_dir() -> Result<PathBuf> {
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => Ok(PathBuf::from(home)),
        _ => bail!("cannot determine home directory: $HOME is not set"),
    }
}

/// Return the per-user data directory: `~/.autocomplete-rs/`.
///
/// Holds the database, logs, and the daemon socket, and is kept at mode 0700.
pub(crate) fn data_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".autocomplete-rs"))
}

/// Return the default daemon socket path: `~/.autocomplete-rs/daemon.sock`.
///
/// Deliberately not under `/tmp`: that directory is world-writable, so another local user
/// could pre-create the socket path and accept our clients' connections before the daemon
/// binds.
pub fn default_socket_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("daemon.sock"))
}

/// What [`ensure_private_dir`] did to a directory.
///
/// Returned rather than logged: this runs before the tracing subscriber exists during
/// logging setup, so the caller decides when it is safe to report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirAction {
    /// Did not exist; created at 0700.
    Created,
    /// Already private; left alone.
    AlreadyPrivate,
    /// Ours but group/other accessible; tightened to 0700.
    Tightened { previous_mode: u32 },
}

/// Whether a directory that is not already private may be modified in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repair {
    /// Tighten to 0700. Only for directories inside our own data directory.
    Yes,
    /// Report the problem and leave the directory alone.
    No,
}

/// Whether `dir` lives inside our data directory, and so is ours to tighten.
///
/// Two ways a path can look contained without being contained, both refused:
/// a `..` component walks out lexically (`~/.autocomplete-rs/../.ssh`), and a symlink
/// walks out at resolution time. `Path::starts_with` is component-wise, so a sibling
/// like `.autocomplete-rs-evil` is already excluded.
fn within_data_dir(dir: &Path) -> bool {
    use std::path::Component;

    if dir.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let Ok(root) = data_dir() else {
        return false;
    };
    // Both must resolve, or the comparison is between spellings rather than locations.
    match (resolve_existing_prefix(dir), resolve_existing_prefix(&root)) {
        (Some(dir), Some(root)) => dir.starts_with(root),
        _ => false,
    }
}

/// Canonicalize the deepest ancestor of `path` that exists, then re-append the rest.
///
/// `canonicalize` fails outright on a path that does not exist yet, which is the normal
/// case here — the directory is about to be created. Returns `None` when no ancestor can
/// be resolved, so callers fail closed instead of silently comparing an unresolved path:
/// `dir` and `./dir` must not reach different security decisions.
///
/// Cannot resolve a `..`; [`within_data_dir`] rejects those before calling this.
fn resolve_existing_prefix(path: &Path) -> Option<PathBuf> {
    let absolute;
    let mut cursor: &Path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir().ok()?.join(path);
        &absolute
    };

    let mut tail = Vec::new();
    loop {
        if let Ok(resolved) = cursor.canonicalize() {
            let mut out = resolved;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return Some(out);
        }
        match (cursor.file_name(), cursor.parent()) {
            (Some(name), Some(parent)) => {
                tail.push(name.to_os_string());
                cursor = parent;
            }
            _ => return None,
        }
    }
}

/// What [`inspect_dir`] concluded about an existing directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirVerdict {
    /// Private and ours; use as-is.
    Accept,
    /// Ours but group/other accessible.
    NeedsRepair { mode: u32 },
    /// Someone else's directory on our path.
    ForeignOwner { uid: u32 },
}

/// Classify a directory from its mode and owner.
///
/// Ownership is checked before permissions: a directory that is already 0700 but belongs
/// to someone else is still theirs, and its owner can replace whatever we put inside it.
pub(crate) fn inspect_dir(mode: u32, dir_uid: u32, our_uid: u32) -> DirVerdict {
    if dir_uid != our_uid {
        return DirVerdict::ForeignOwner { uid: dir_uid };
    }
    if mode & 0o077 == 0 {
        DirVerdict::Accept
    } else {
        DirVerdict::NeedsRepair { mode }
    }
}

/// Ensure `dir` exists, belongs to us, and is not group/other accessible.
///
/// A missing directory is created at mode 0700. A directory owned by another user, or
/// reached through a symlink, is always rejected. One that is ours but too permissive is
/// tightened only when it lives inside our data directory; a path the user chose elsewhere
/// is reported instead, since chmodding it could revoke access to unrelated files.
///
/// Returns one entry per directory acted on, outermost first. For a path inside the data
/// directory that is every level from the data root down, so no intermediate level is left
/// group-traversable — a writable one there would re-open the swap window `ensure_one`
/// closes. For a path outside it, only the leaf.
pub(crate) fn ensure_private_dir(dir: &Path) -> Result<Vec<(PathBuf, DirAction)>> {
    let mut actions = Vec::new();

    if within_data_dir(dir) {
        for level in levels_from_data_root(dir) {
            actions.push((level.clone(), ensure_one(&level, Repair::Yes)?));
        }
        return Ok(actions);
    }

    actions.push((dir.to_path_buf(), ensure_one(dir, Repair::No)?));
    Ok(actions)
}

/// Every directory from the data root down to `dir`, inclusive, outermost first.
///
/// `dir` is known to be inside the data root; if the relative step cannot be computed the
/// root alone is returned rather than silently skipping levels.
fn levels_from_data_root(dir: &Path) -> Vec<PathBuf> {
    let Ok(root) = data_dir() else {
        return vec![dir.to_path_buf()];
    };
    let (Some(resolved_dir), Some(resolved_root)) =
        (resolve_existing_prefix(dir), resolve_existing_prefix(&root))
    else {
        return vec![dir.to_path_buf()];
    };
    let Ok(relative) = resolved_dir.strip_prefix(&resolved_root) else {
        return vec![root];
    };

    let mut levels = vec![root.clone()];
    let mut cursor = root;
    for component in relative.components() {
        cursor = cursor.join(component);
        levels.push(cursor.clone());
    }
    levels
}

/// Ensure a single directory, without considering its ancestors.
///
/// Opens with `O_NOFOLLOW | O_DIRECTORY` and works through that descriptor, so the
/// directory inspected is the directory modified. Checking a path and then chmod'ing the
/// same path is a race: both `stat` and `chmod` follow symlinks, so anything able to swap
/// the name for a symlink between the two calls redirects the chmod. That window is
/// small — measured as winnable in 6 ms — but it is real, and `O_NOFOLLOW` closes it
/// along with the dangling-symlink and symlinked-root cases.
fn ensure_one(dir: &Path, repair: Repair) -> Result<DirAction> {
    let (fd, created) = match open_dir_nofollow(dir) {
        Ok(fd) => (fd, false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            create_private_dir(dir)?;
            let fd = open_dir_nofollow(dir).with_context(|| {
                format!(
                    "failed to open directory {} after creating it",
                    dir.display()
                )
            })?;
            (fd, true)
        }
        Err(e) => {
            return Err(e).context(match symlink_kind(dir) {
                Some(kind) => format!("refusing to use {} — it is a {kind}", dir.display()),
                None => format!("failed to open directory {}", dir.display()),
            });
        }
    };

    let metadata = fd
        .metadata()
        .with_context(|| format!("failed to stat directory {}", dir.display()))?;

    let mode = metadata.permissions().mode() & 0o777;
    match inspect_dir(mode, metadata_uid(&metadata), current_uid()) {
        DirVerdict::Accept if created => Ok(DirAction::Created),
        DirVerdict::Accept => Ok(DirAction::AlreadyPrivate),
        DirVerdict::ForeignOwner { uid } => bail!(
            "directory {} is owned by uid {}, not by us (uid {}) — \
             refusing to use it for private data",
            dir.display(),
            uid,
            current_uid()
        ),
        DirVerdict::NeedsRepair { mode } if repair == Repair::Yes => {
            fchmod_0700(&fd).with_context(|| {
                format!(
                    "failed to tighten permissions on {} from {:o} to 0700",
                    dir.display(),
                    mode
                )
            })?;
            Ok(DirAction::Tightened {
                previous_mode: mode,
            })
        }
        DirVerdict::NeedsRepair { mode } => bail!(
            "directory {} has permissions {:o} and must not be group/other accessible; \
             tighten it with: chmod 700 {}",
            dir.display(),
            mode,
            dir.display()
        ),
    }
}

/// Open `dir` without following a final-component symlink.
///
/// A symlink or a non-directory yields the platform's `ELOOP`/`ENOTDIR` rather than
/// silently operating on whatever the link points at.
fn open_dir_nofollow(dir: &Path) -> std::io::Result<fs::File> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::FromRawFd;

    let path = CString::new(dir.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::other("directory path contains an interior NUL"))?;

    // SAFETY: `path` is a valid NUL-terminated C string that outlives the call, and the
    // returned descriptor is immediately handed to `File`, which owns and closes it.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `fd` is a fresh, valid descriptor not owned by anything else.
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

/// Set an open directory to mode 0700 through its descriptor.
fn fchmod_0700(dir: &fs::File) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;

    // SAFETY: `dir` owns a valid open descriptor for the duration of the call.
    if unsafe { libc::fchmod(dir.as_raw_fd(), 0o700) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Create `dir` and any missing ancestors at mode 0700.
fn create_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .with_context(|| format!("failed to create directory {}", dir.display()))
}

/// Describe what `path` is, when opening it as a directory failed.
fn symlink_kind(path: &Path) -> Option<&'static str> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() {
        Some("symlink")
    } else if !metadata.is_dir() {
        Some("file, not a directory")
    } else {
        None
    }
}

/// Ensure the parent directory of `path` exists and is private.
pub(crate) fn ensure_private_parent(path: &Path) -> Result<Vec<(PathBuf, DirAction)>> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .with_context(|| format!("path {} has no parent directory", path.display()))?;
    ensure_private_dir(dir)
}

/// Emit a warning for each directory whose permissions were tightened.
///
/// Separate from the work so callers that run before the tracing subscriber exists can
/// defer it — a warning emitted then would go nowhere.
pub(crate) fn log_dir_actions(actions: &[(PathBuf, DirAction)]) {
    for (path, action) in actions {
        if let DirAction::Tightened { previous_mode } = action {
            tracing::warn!(
                path = %path.display(),
                previous_mode = format!("{previous_mode:o}"),
                "directory was group/other accessible and has been tightened to 0700"
            );
        }
    }
}

/// Reject a socket path the platform cannot bind, before `bind` fails obscurely.
pub(crate) fn check_socket_path_len(path: &Path) -> Result<()> {
    let len = path.as_os_str().as_encoded_bytes().len();
    if len >= MAX_SOCKET_PATH_LEN {
        bail!(
            "socket path is {} bytes, which exceeds the {}-byte limit for Unix sockets: {}. \
             Set AUTOCOMPLETE_RS_SOCKET to a shorter path.",
            len,
            MAX_SOCKET_PATH_LEN - 1,
            path.display()
        );
    }
    Ok(())
}

fn metadata_uid(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.uid()
}

/// Real user id of this process.
fn current_uid() -> u32 {
    // SAFETY: `getuid` takes no arguments, cannot fail, and has no side effects.
    unsafe { libc::getuid() }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Run `f` with `$HOME` set to `value`, restoring the previous value afterwards.
    ///
    /// Serialised through a mutex because the environment is process-global and the test
    /// binary is threaded.
    pub(crate) fn with_home<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        use std::sync::{Mutex, OnceLock};
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let previous = std::env::var_os("HOME");
        // SAFETY: every write to HOME in this test binary is serialised by ENV_LOCK.
        unsafe {
            match value {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
        let result = f();
        // SAFETY: as above; restores the value observed under the same guard.
        unsafe {
            match previous {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
        result
    }

    #[test]
    fn default_socket_path_is_under_the_data_dir() {
        let socket = with_home(Some("/home/someone"), || default_socket_path().unwrap());
        assert_eq!(
            socket,
            PathBuf::from("/home/someone/.autocomplete-rs/daemon.sock")
        );
    }

    #[test]
    fn missing_home_is_an_error_not_a_tmp_fallback() {
        // Falling back to /tmp would put the socket back in the world-writable directory
        // this path exists to avoid.
        let err = with_home(None, || default_socket_path().unwrap_err());
        assert!(
            err.to_string().contains("$HOME is not set"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn empty_home_is_an_error_too() {
        let err = with_home(Some(""), || default_socket_path().unwrap_err());
        assert!(
            err.to_string().contains("$HOME is not set"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn inspect_accepts_private_dir_we_own() {
        assert_eq!(inspect_dir(0o700, 501, 501), DirVerdict::Accept);
    }

    #[test]
    fn inspect_flags_group_access_not_only_other() {
        // The mask is 0o077; testing only other-accessible modes leaves the group half
        // unconstrained, so a chmod 750 directory could pass as private.
        assert_eq!(
            inspect_dir(0o750, 501, 501),
            DirVerdict::NeedsRepair { mode: 0o750 }
        );
        assert_eq!(
            inspect_dir(0o710, 501, 501),
            DirVerdict::NeedsRepair { mode: 0o710 }
        );
    }

    #[test]
    fn inspect_flags_group_or_other_access() {
        assert_eq!(
            inspect_dir(0o755, 501, 501),
            DirVerdict::NeedsRepair { mode: 0o755 }
        );
        assert_eq!(
            inspect_dir(0o701, 501, 501),
            DirVerdict::NeedsRepair { mode: 0o701 }
        );
    }

    #[test]
    fn inspect_rejects_foreign_owner_even_when_private() {
        // The branch a filesystem test cannot reach: an unprivileged process cannot chown
        // a directory to another uid to stage it.
        assert_eq!(
            inspect_dir(0o700, 0, 501),
            DirVerdict::ForeignOwner { uid: 0 }
        );
        assert_eq!(
            inspect_dir(0o755, 0, 501),
            DirVerdict::ForeignOwner { uid: 0 }
        );
    }

    #[test]
    fn ensure_private_dir_creates_with_mode_0700() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let dir = data_dir().unwrap().join("nested");

            let actions = ensure_private_dir(&dir).expect("should create nested dir");
            assert!(
                actions
                    .iter()
                    .any(|(p, a)| *p == dir && *a == DirAction::Created),
                "expected a Created action for the leaf: {actions:?}"
            );

            let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "expected 0700, got {mode:o}");

            let again = ensure_private_dir(&dir).expect("should accept existing private dir");
            assert!(
                again
                    .iter()
                    .any(|(p, a)| *p == dir && *a == DirAction::AlreadyPrivate),
                "second call should be a no-op: {again:?}"
            );
        });
    }

    #[test]
    fn ensure_private_dir_repairs_inside_our_data_directory() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let root = data_dir().unwrap();
            let dir = root.join("logs");
            fs::create_dir_all(&dir).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

            let actions = ensure_private_dir(&dir).expect("our own directories are repairable");

            // Securing something inside the data dir secures the data dir on the way down.
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700,
                "the data root must be tightened too"
            );
            assert_eq!(
                fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                actions.len(),
                2,
                "expected the root and the leaf to be reported: {actions:?}"
            );
            assert!(
                actions.iter().all(|(_, a)| matches!(
                    a,
                    DirAction::Tightened {
                        previous_mode: 0o755
                    }
                )),
                "both should report the previous mode: {actions:?}"
            );
        });
    }

    #[test]
    fn ensure_private_dir_reports_instead_of_mutating_a_path_outside_our_data_dir() {
        let home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        with_home(Some(home.path().to_str().unwrap()), || {
            let dir = elsewhere.path().join("user-chosen");
            fs::create_dir(&dir).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

            let err = ensure_private_dir(&dir)
                .expect_err("a 0755 dir the user chose must be reported, not chmodded");
            assert!(
                err.to_string().contains("chmod 700"),
                "error should tell the user how to fix it: {err}"
            );
            assert_eq!(
                fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o755,
                "directory must be left untouched"
            );
        });
    }

    #[test]
    fn ensure_private_dir_rejects_a_file_in_the_way() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("not-a-dir");
        fs::write(&path, b"contents").unwrap();

        let err = ensure_private_dir(&path).expect_err("a file must not pass as a directory");
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn ensure_private_parent_creates_the_socket_directory() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let sock = default_socket_path().unwrap();

            ensure_private_parent(&sock).expect("should create the socket's parent");

            let mode = fs::metadata(sock.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o700, "expected 0700, got {mode:o}");
        });
    }

    #[test]
    fn parent_traversal_behind_a_missing_component_is_not_inside_our_data_dir() {
        // The shape that actually needs the ParentDir guard. When every component exists,
        // `canonicalize` resolves the `..` on its own and the guard is never exercised —
        // but a `..` behind a component that does not exist yet (the normal case here,
        // since the directory is about to be created) leaves the path unresolved.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let root = data_dir().unwrap();
            fs::create_dir_all(&root).unwrap();
            let victim = home.join("shared");
            fs::create_dir_all(&victim).unwrap();
            fs::set_permissions(&victim, fs::Permissions::from_mode(0o755)).unwrap();

            let escape = root.join("nope").join("..").join("..").join("shared");
            assert!(
                !within_data_dir(&escape),
                "`..` must not count as contained"
            );

            ensure_private_dir(&escape).expect_err("must be reported, not tightened");
            assert_eq!(
                fs::metadata(&victim).unwrap().permissions().mode() & 0o777,
                0o755,
                "a directory outside the data dir must be left untouched"
            );
        });
    }

    #[test]
    fn parent_traversal_with_every_component_present_is_also_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            fs::create_dir_all(home.join(".ssh")).unwrap();
            let escape = data_dir().unwrap().join("..").join(".ssh");
            assert!(!within_data_dir(&escape));
        });
    }

    #[test]
    fn a_symlinked_data_root_is_refused() {
        // `~/.autocomplete-rs -> ~/Documents` used to canonicalize through the link on both
        // sides, so containment said "ours" and Documents was chmodded to 0700.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let victim = home.join("Documents");
            fs::create_dir_all(&victim).unwrap();
            fs::set_permissions(&victim, fs::Permissions::from_mode(0o755)).unwrap();
            std::os::unix::fs::symlink(&victim, home.join(".autocomplete-rs")).unwrap();

            let err = ensure_private_dir(&data_dir().unwrap())
                .expect_err("a symlinked data root must be refused");
            assert!(
                format!("{err:#}").contains("symlink"),
                "error should name the cause: {err:#}"
            );
            assert_eq!(
                fs::metadata(&victim).unwrap().permissions().mode() & 0o777,
                0o755,
                "the symlink target must be left untouched"
            );
        });
    }

    #[test]
    fn a_symlink_where_a_subdirectory_belongs_is_refused() {
        // The swap target in the check-then-chmod race: without O_NOFOLLOW the chmod
        // follows this link and tightens whatever it points at. A timing test proves the
        // race is closed but is flaky in CI; this pins the mechanism deterministically.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let root = data_dir().unwrap();
            fs::create_dir_all(&root).unwrap();
            let victim = home.join("victim");
            fs::create_dir_all(&victim).unwrap();
            fs::set_permissions(&victim, fs::Permissions::from_mode(0o755)).unwrap();

            let logs = root.join("logs");
            std::os::unix::fs::symlink(&victim, &logs).unwrap();

            ensure_private_dir(&logs).expect_err("a symlink must not be followed");
            assert_eq!(
                fs::metadata(&victim).unwrap().permissions().mode() & 0o777,
                0o755,
                "the symlink target must be left untouched"
            );
        });
    }

    #[test]
    fn a_relative_path_reaches_the_same_verdict_as_its_dotted_spelling() {
        // `dir` and `./dir` must not resolve to different security decisions.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let root = data_dir().unwrap();
            fs::create_dir_all(&root).unwrap();
            let restore = std::env::current_dir().unwrap();
            std::env::set_current_dir(&root).unwrap();

            let bare = within_data_dir(Path::new("child"));
            let dotted = within_data_dir(Path::new("./child"));

            std::env::set_current_dir(restore).unwrap();
            assert_eq!(bare, dotted, "spelling must not change containment");
            assert!(bare, "a relative path under the data dir is inside it");
        });
    }

    #[test]
    fn intermediate_levels_are_tightened_too() {
        // A group-writable level between the root and the leaf would re-open the window
        // that opening with O_NOFOLLOW closes.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let root = data_dir().unwrap();
            let mid = root.join("a");
            let leaf = mid.join("b");
            fs::create_dir_all(&leaf).unwrap();
            for d in [&root, &mid, &leaf] {
                fs::set_permissions(d, fs::Permissions::from_mode(0o755)).unwrap();
            }

            ensure_private_dir(&leaf).expect("our own tree is repairable");

            for d in [&root, &mid, &leaf] {
                assert_eq!(
                    fs::metadata(d).unwrap().permissions().mode() & 0o777,
                    0o700,
                    "{} was left permissive",
                    d.display()
                );
            }
        });
    }

    #[test]
    fn a_non_default_name_inside_the_data_dir_is_repaired() {
        // Pins the deliberate widening: containment, not equality with a known default.
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        with_home(Some(home.to_str().unwrap()), || {
            let root = data_dir().unwrap();
            let custom = root.join("custom-sockets");
            fs::create_dir_all(&custom).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
            fs::set_permissions(&custom, fs::Permissions::from_mode(0o755)).unwrap();

            ensure_private_dir(&custom).expect("a path inside our data dir is ours to fix");

            assert_eq!(
                fs::metadata(&custom).unwrap().permissions().mode() & 0o777,
                0o700
            );
        });
    }

    #[test]
    fn a_symlink_out_of_the_data_dir_is_not_inside_it() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let root = data_dir().unwrap();
            fs::create_dir_all(&root).unwrap();
            let link = root.join("escape");
            std::os::unix::fs::symlink(outside.path(), &link).unwrap();

            assert!(
                !within_data_dir(&link),
                "a symlink resolving outside the data dir must not count as contained"
            );
        });
    }

    #[test]
    fn a_sibling_directory_is_not_inside_our_data_dir() {
        // starts_with is component-wise, so `.autocomplete-rs-evil` must not match.
        let tmp = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let sibling = tmp.path().join(".autocomplete-rs-evil");
            assert!(!within_data_dir(&sibling));
        });
    }

    #[test]
    fn the_data_dir_and_its_children_are_inside_it() {
        let tmp = tempfile::tempdir().unwrap();
        with_home(Some(tmp.path().to_str().unwrap()), || {
            let root = data_dir().unwrap();
            assert!(within_data_dir(&root));
            assert!(within_data_dir(&root.join("logs")));
        });
    }

    #[test]
    fn log_dir_actions_warns_about_a_tightened_directory() {
        // The reporting half of the refactor: `ensure_private_dir` returns actions instead
        // of logging so the caller can warn once a subscriber exists. Without this, making
        // `log_dir_actions` a no-op passes the whole suite and the user is never told their
        // directory was chmodded.
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl tracing_subscriber::fmt::MakeWriter<'_> for Capture {
            type Writer = Self;
            fn make_writer(&self) -> Self::Writer {
                self.clone()
            }
        }

        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            log_dir_actions(&[
                (
                    PathBuf::from("/home/u/.autocomplete-rs"),
                    DirAction::Tightened {
                        previous_mode: 0o755,
                    },
                ),
                (PathBuf::from("/home/u/quiet"), DirAction::AlreadyPrivate),
            ]);
        });

        let logged = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        assert!(
            logged.contains("/home/u/.autocomplete-rs"),
            "the warning must name the directory: {logged}"
        );
        assert!(
            logged.contains("755"),
            "the warning must report the previous mode: {logged}"
        );
        assert!(
            !logged.contains("quiet"),
            "an already-private directory must not be reported: {logged}"
        );
    }

    #[test]
    fn socket_path_length_boundary_matches_what_bind_accepts() {
        // Measured on macOS: a 103-byte path binds, 104 fails with "path too long".
        let longest_ok = PathBuf::from("x".repeat(MAX_SOCKET_PATH_LEN - 1));
        check_socket_path_len(&longest_ok).expect("the longest usable path must be accepted");

        let too_long = PathBuf::from("x".repeat(MAX_SOCKET_PATH_LEN));
        let err =
            check_socket_path_len(&too_long).expect_err("a path at the limit must be rejected");
        assert!(
            err.to_string().contains("AUTOCOMPLETE_RS_SOCKET"),
            "error should be actionable: {err}"
        );
    }
}
