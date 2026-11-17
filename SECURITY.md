---
status: active
owner: Andrii Simonov
last_updated: 2026-11-17
lang: en
---

# Security Policy

**Language:** **English** | [Русский](SECURITY.ru.md)

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |
| < 0.1   | ❌        |

## Reporting a vulnerability

If you discover a security vulnerability in Polygram, please report it responsibly.

### How to report

1. Do **not** create a public GitHub issue for security vulnerabilities.
2. Send details to the project maintainer via GitHub security advisories or email.
3. Include: description, steps to reproduce, impact, and suggested fix (if any).

### What to expect

- Confirmation of receipt within 48 hours.
- Initial assessment within 7 days.
- Regular updates on remediation progress.

## Scope

- Core Rust engine
- egui GUI / canvas
- HTTP API and MCP server
- CLI terminal

## Out of scope

- Third-party extensions in `/extensions`
- User-deployed infrastructure

## Acknowledgments

Security researchers who responsibly disclose vulnerabilities will be acknowledged in our release notes (with permission).