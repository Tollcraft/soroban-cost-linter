# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Automated Security Audits

Dependencies are automatically audited for security vulnerabilities, license compliance, and unmaintained crates using `cargo-deny`. In addition to running on every push and pull request to the `main` branch, the audit workflow runs on a daily schedule (`0 6 * * *`) against the default branch to catch newly disclosed vulnerabilities promptly.

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Instead, please report them to the maintainers privately:
- Contact via email: security@tollcraft.org
- Or via Telegram: [Tollcraft](https://t.me/+Gflo5jZStw1jMjE0)

You should receive a response within 48 hours. If the issue is confirmed as a vulnerability, we will open a private security advisory and work on a patch before public disclosure.
