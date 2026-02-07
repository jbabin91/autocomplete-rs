use std::path::PathBuf;

/// Resolve the user's home directory from `$HOME`, falling back to `/tmp`.
pub(crate) fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
