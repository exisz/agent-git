# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in agent-git, please report it responsibly:

1. **Do NOT open a public issue**
2. Use [GitHub Security Advisories](https://github.com/exisz/agent-git/security/advisories/new) to report the vulnerability privately
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Response Timeline

- **Acknowledgment:** Within 48 hours
- **Assessment:** Within 1 week
- **Fix:** Depending on severity, typically within 2 weeks

## Scope

agent-git interacts with your filesystem (tracking clone locations) and shells out to `git`. Security concerns include:

- Path traversal in registry operations
- Shell injection via URL or path arguments
- Registry file tampering

We take these seriously and will address reported issues promptly.
