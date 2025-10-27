# Security Policy

## Supported Versions

autocomplete-rs is currently in pre-release development (v0.1.0-alpha). Security
updates will be provided for:

| Version | Supported          |
| ------- | ------------------ |
| main    | :white_check_mark: |
| < 0.1.0 | :x:                |

Once we reach v1.0.0, this policy will be updated to reflect stable release
support.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report security vulnerabilities by:

1. **Email:** Open a private security advisory via GitHub's security tab, or
2. **Direct Contact:** Contact the maintainer directly

**Please include:**

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**What to expect:**

- **Initial Response:** Within 48 hours acknowledging receipt
- **Status Update:** Within 7 days with assessment and planned fix timeline
- **Resolution:** Security fixes will be prioritized and released as soon as
  possible

**Disclosure Policy:**

- Please allow reasonable time for a fix before public disclosure
- We aim to fix critical vulnerabilities within 14 days
- You will be credited in the release notes (unless you prefer anonymity)

## Security Considerations

autocomplete-rs is designed with security in mind:

### Unix Socket Security

- Socket permissions set to 0600 (user-only access)
- No other users can connect to your daemon
- Standard Unix DAC (Discretionary Access Control)

### Input Validation

- Buffer length limits (max 10,000 chars)
- Request size limits (max 100KB)
- Cursor position validation
- No code execution from user input

### Resource Limits

- Connection limits (max 100 concurrent)
- Request timeouts (100ms)
- Memory limits (~50MB max)

### No Privilege Escalation

- Runs as normal user
- No setuid/setgid
- No root access required

### Spec Safety

- Specs are data, not code
- No dynamic evaluation
- No user-provided specs in Phase 1

## Known Limitations

### Current (Phase 1):

- No encryption on Unix socket (local-only communication)
- No authentication (relies on file permissions)
- No rate limiting per connection

### Future Improvements (Planned):

- Custom spec sandboxing (Phase 3+)
- Generator command whitelisting (Phase 2)
- Optional spec signature verification (Post-1.0)

## Security Best Practices for Users

1. **Keep Updated:** Use the latest version
2. **Verify Source:** Only install from official sources
3. **Check Permissions:** Ensure socket is 0600
4. **Review Logs:** Check for unexpected activity (`RUST_LOG=debug`)
5. **Report Issues:** Report any suspicious behavior immediately

## Contact

- **Security Issues:** Use GitHub Security Advisories (preferred)
- **General Questions:**
  [GitHub Discussions](https://github.com/jacebabin/autocomplete-rs/discussions)
- **Maintainer:** Jace Babin ([@jacebabin](https://github.com/jacebabin))

---

Thank you for helping keep autocomplete-rs safe!
