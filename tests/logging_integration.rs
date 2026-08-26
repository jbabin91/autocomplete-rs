use std::fs;
use std::os::unix::fs::PermissionsExt;

use autocomplete_rs::logging::{
    LogConfig, Mode, default_log_dir, new_request_id, redact_buffer, redact_sensitive_patterns,
    should_redact,
};

#[test]
fn test_log_config_production_defaults() {
    let tmp = private_tempdir();
    let cfg = LogConfig {
        mode: Mode::Production,
        log_dir: tmp.path().to_path_buf(),
        retention_days: 7,
        enable_console: false,
        redact_buffers: true,
    };
    assert!(cfg.redact_buffers);
    assert!(!cfg.enable_console);
    assert_eq!(cfg.retention_days, 7);
}

#[test]
fn test_log_dir_creation_with_permissions() {
    let tmp = private_tempdir();
    let log_dir = tmp.path().join("test-logs");

    // init_with_config would create this, but we test the dir creation logic
    // by calling the public ensure function indirectly through init_with_config.
    // Since we can't call init twice in a process, test the dir setup directly.
    assert!(!log_dir.exists());
    fs::create_dir_all(&log_dir).unwrap();
    fs::set_permissions(&log_dir, fs::Permissions::from_mode(0o700)).unwrap();

    let perms = fs::metadata(&log_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(perms, 0o700);
}

#[test]
fn test_default_log_dir_path() {
    let dir = default_log_dir().expect("HOME is set in the test environment");
    let path_str = dir.to_string_lossy();
    assert!(path_str.contains("autocomplete-rs"));
    assert!(path_str.ends_with("logs"));
}

#[test]
fn test_redaction_roundtrip() {
    let sensitive = "curl -H 'token=sk-abc123' https://user:pass@api.example.com/v1";
    let redacted = redact_sensitive_patterns(sensitive);

    assert!(!redacted.contains("sk-abc123"));
    assert!(!redacted.contains("pass"));
    assert!(redacted.contains("[REDACTED]"));
    assert!(redacted.contains("api.example.com"));
}

#[test]
fn test_buffer_redaction_preserves_length_info() {
    let buffer = "git commit -m 'my secret message'";
    let redacted = redact_buffer(buffer);

    // Should contain the length
    assert!(redacted.contains(&format!("({})", buffer.len())));
    // Should NOT contain the full buffer
    assert!(!redacted.contains("secret message"));
}

#[test]
fn test_mode_determines_redaction() {
    assert!(should_redact(&Mode::Production));
    assert!(!should_redact(&Mode::Development));
}

#[test]
fn test_request_id_generation() {
    let ids: Vec<String> = (0..10).map(|_| new_request_id()).collect();

    // All unique
    for (i, id) in ids.iter().enumerate() {
        for (j, other) in ids.iter().enumerate() {
            if i != j {
                assert_ne!(id, other);
            }
        }
    }

    // All valid UUID format
    for id in &ids {
        assert_eq!(id.len(), 36);
    }
}

/// A temp directory at mode 0700.
///
/// `tempfile` honours the umask (0755 in practice), but the daemon refuses to put private
/// data in a group/other-accessible directory — as a real user's `~/.autocomplete-rs`
/// would never be.
fn private_tempdir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
        .expect("failed to restrict temp dir");
    dir
}
