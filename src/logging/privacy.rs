use std::fmt;

use super::Mode;

/// Redact a shell buffer for logging, preserving first 3 and last 3 chars.
///
/// For buffers with 6+ chars: `git com...and (15)`.
/// For shorter buffers: `*** (3)`.
pub fn redact_buffer(buffer: &str) -> String {
    let chars: Vec<char> = buffer.chars().collect();
    let len = chars.len();
    if len >= 6 {
        let prefix: String = chars[..3].iter().collect();
        let suffix: String = chars[len - 3..].iter().collect();
        format!("{prefix}...{suffix} ({len})")
    } else {
        format!("*** ({len})")
    }
}

/// Redact well-known sensitive patterns from a string without regex.
///
/// Handles:
/// - `password=`, `pwd=`, `passwd=` key-value pairs
/// - `api_key=`, `apikey=`, `token=`, `secret=` key-value pairs
/// - URL credentials `://user:pass@host` → `://[REDACTED]@host`
/// - `export VAR=value` for sensitive-looking variable names
pub fn redact_sensitive_patterns(input: &str) -> String {
    let mut result = input.to_string();

    // Redact key=value patterns for sensitive keys
    for key in &[
        "password=",
        "pwd=",
        "passwd=",
        "api_key=",
        "apikey=",
        "token=",
        "secret=",
    ] {
        result = redact_key_value(&result, key);
    }

    // Redact URL credentials: ://user:pass@host → ://[REDACTED]@host
    result = redact_url_credentials(&result);

    // Redact export SECRET_VAR=value patterns
    result = redact_export_secrets(&result);

    result
}

/// Whether the current mode should redact buffers.
pub fn should_redact(mode: &Mode) -> bool {
    match mode {
        Mode::Production => true,
        Mode::Development => false,
        Mode::Troubleshooting => std::env::var("AUTOCOMPLETE_LOG_FULL_BUFFERS")
            .map(|v| v != "1")
            .unwrap_or(true),
    }
}

/// A wrapper that redacts its inner value on Display/Debug.
pub struct RedactedField<T> {
    inner: T,
    redacted: bool,
}

impl<T> RedactedField<T> {
    /// Create a new redacted field. If `redacted` is true, display shows `[REDACTED]`.
    pub fn new(inner: T, redacted: bool) -> Self {
        Self { inner, redacted }
    }
}

impl<T: fmt::Display> fmt::Display for RedactedField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.redacted {
            write!(f, "[REDACTED]")
        } else {
            self.inner.fmt(f)
        }
    }
}

impl<T: fmt::Display> fmt::Debug for RedactedField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.redacted {
            write!(f, "[REDACTED]")
        } else {
            write!(f, "{}", self.inner)
        }
    }
}

/// Redact key=value where the key matches (case-insensitive).
fn redact_key_value(input: &str, key: &str) -> String {
    let lower = input.to_lowercase();
    let key_lower = key.to_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let input_bytes = input.as_bytes();

    while i < input.len() {
        if lower[i..].starts_with(&key_lower) {
            // Copy the original-case key
            result.push_str(&input[i..i + key.len()]);
            i += key.len();
            // Skip the value until whitespace, '&', or end
            while i < input.len() {
                let b = input_bytes[i];
                if b == b' ' || b == b'\t' || b == b'&' || b == b'\n' || b == b'\r' {
                    break;
                }
                i += 1;
            }
            result.push_str("[REDACTED]");
        } else {
            // Advance one character (UTF-8 safe via char_indices isn't needed since
            // we're searching for ASCII keys; just push the byte as-is)
            result.push(input[i..].chars().next().unwrap());
            i += input[i..].chars().next().unwrap().len_utf8();
        }
    }

    result
}

/// Redact `://user:pass@host` → `://[REDACTED]@host`.
fn redact_url_credentials(input: &str) -> String {
    let marker = "://";
    let mut result = String::with_capacity(input.len());
    let mut search_from = 0;

    while let Some(pos) = input[search_from..].find(marker) {
        let abs_pos = search_from + pos;
        let after_marker = abs_pos + marker.len();
        result.push_str(&input[search_from..after_marker]);

        // Look for @ in the rest (but only before the next / or end)
        if let Some(rest) = input.get(after_marker..) {
            let segment_end = rest.find('/').unwrap_or(rest.len());
            let segment = &rest[..segment_end];

            if segment.contains('@') {
                // Has credentials — find the @ and redact everything before it
                let at_pos = segment.rfind('@').unwrap();
                result.push_str("[REDACTED]");
                result.push_str(&segment[at_pos..]);
                search_from = after_marker + segment_end;
            } else {
                // No credentials — copy as-is
                search_from = after_marker;
            }
        } else {
            search_from = input.len();
        }
    }

    result.push_str(&input[search_from..]);
    result
}

/// Redact `export SENSITIVE_VAR=value` patterns.
///
/// A variable name is considered sensitive if it contains (case-insensitive):
/// `SECRET`, `TOKEN`, `PASSWORD`, `KEY`, `CREDENTIAL`, `AUTH`.
fn redact_export_secrets(input: &str) -> String {
    const SENSITIVE_WORDS: &[&str] = &["SECRET", "TOKEN", "PASSWORD", "KEY", "CREDENTIAL", "AUTH"];

    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(pos) = remaining.find("export ") {
        result.push_str(&remaining[..pos]);
        let export_start = pos;
        let after_export = &remaining[export_start + 7..]; // skip "export "

        // Find the variable name (up to =)
        if let Some(eq_pos) = after_export.find('=') {
            let var_name = after_export[..eq_pos].trim();
            let var_upper = var_name.to_uppercase();
            let is_sensitive = SENSITIVE_WORDS.iter().any(|word| var_upper.contains(word));

            if is_sensitive {
                // Copy "export VAR=" then redact the value
                result.push_str(&remaining[export_start..export_start + 7 + eq_pos + 1]);
                let value_start = &after_export[eq_pos + 1..];
                // Skip value until whitespace or end
                let value_end = value_start
                    .find([' ', '\t', '\n'])
                    .unwrap_or(value_start.len());
                result.push_str("[REDACTED]");
                remaining = &value_start[value_end..];
            } else {
                // Not sensitive — copy "export VAR=..." as-is
                let line_end = after_export
                    .find('\n')
                    .map(|p| p + 1)
                    .unwrap_or(after_export.len());
                result.push_str(&remaining[export_start..export_start + 7 + line_end]);
                remaining = &after_export[line_end..];
            }
        } else {
            // No = found, just copy "export " and continue
            result.push_str("export ");
            remaining = after_export;
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_buffer_long() {
        let result = redact_buffer("git commit -m 'secret message'");
        assert!(result.starts_with("git"));
        assert!(result.contains("..."));
        assert!(result.ends_with(')'));
        assert!(result.contains("(30)"));
    }

    #[test]
    fn test_redact_buffer_short() {
        assert_eq!(redact_buffer("ls"), "*** (2)");
        assert_eq!(redact_buffer(""), "*** (0)");
        assert_eq!(redact_buffer("abcde"), "*** (5)");
    }

    #[test]
    fn test_redact_buffer_exactly_six() {
        let result = redact_buffer("abcdef");
        assert_eq!(result, "abc...def (6)");
    }

    #[test]
    fn test_redact_buffer_unicode() {
        // 4 chars, should be short-redacted
        assert_eq!(redact_buffer("café"), "*** (4)");
        // 7 chars
        let result = redact_buffer("héllo w");
        assert_eq!(result, "hél...o w (7)");
    }

    #[test]
    fn test_redact_sensitive_password() {
        let input = "curl --data password=hunter2 http://example.com";
        let result = redact_sensitive_patterns(input);
        assert!(result.contains("password=[REDACTED]"));
        assert!(!result.contains("hunter2"));
    }

    #[test]
    fn test_redact_sensitive_api_key() {
        let input = "api_key=sk-1234567890";
        let result = redact_sensitive_patterns(input);
        assert_eq!(result, "api_key=[REDACTED]");
    }

    #[test]
    fn test_redact_sensitive_token() {
        let input = "token=abc123&other=safe";
        let result = redact_sensitive_patterns(input);
        assert!(result.contains("token=[REDACTED]"));
        assert!(result.contains("other=safe"));
    }

    #[test]
    fn test_redact_url_credentials() {
        let input = "https://user:s3cret@github.com/repo";
        let result = redact_sensitive_patterns(input);
        assert!(result.contains("://[REDACTED]@github.com"));
        assert!(!result.contains("s3cret"));
    }

    #[test]
    fn test_redact_url_no_credentials() {
        let input = "https://github.com/repo";
        let result = redact_sensitive_patterns(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_redact_export_secret() {
        let input = "export MY_SECRET_KEY=supersecret123";
        let result = redact_sensitive_patterns(input);
        assert!(result.contains("MY_SECRET_KEY=[REDACTED]"));
        assert!(!result.contains("supersecret123"));
    }

    #[test]
    fn test_redact_export_non_secret() {
        let input = "export PATH=/usr/local/bin";
        let result = redact_sensitive_patterns(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_should_redact_modes() {
        assert!(should_redact(&Mode::Production));
        assert!(!should_redact(&Mode::Development));
        // Troubleshooting without env var → redact
        // SAFETY: test-only, single-threaded access to this env var
        unsafe {
            std::env::remove_var("AUTOCOMPLETE_LOG_FULL_BUFFERS");
        }
        assert!(should_redact(&Mode::Troubleshooting));
    }

    #[test]
    fn test_redacted_field_display() {
        let field = RedactedField::new("secret_value", true);
        assert_eq!(format!("{field}"), "[REDACTED]");

        let field = RedactedField::new("visible_value", false);
        assert_eq!(format!("{field}"), "visible_value");
    }

    #[test]
    fn test_redacted_field_debug() {
        let field = RedactedField::new("secret", true);
        assert_eq!(format!("{field:?}"), "[REDACTED]");

        let field = RedactedField::new("visible", false);
        assert_eq!(format!("{field:?}"), "visible");
    }

    #[test]
    fn test_no_sensitive_data_unchanged() {
        let input = "ls -la /home/user";
        assert_eq!(redact_sensitive_patterns(input), input);
    }

    #[test]
    fn test_case_insensitive_key_redaction() {
        let input = "PASSWORD=hunter2";
        let result = redact_sensitive_patterns(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("hunter2"));
    }
}
