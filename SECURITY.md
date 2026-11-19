# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Automated Security Audits

Dependencies are automatically audited for security vulnerabilities, license compliance, and unmaintained crates using `cargo-deny`. The audit workflow runs when dependency-related files change (`Cargo.lock`, `Cargo.toml`, `deny.toml`) on pushes to `main`, on a daily schedule at 06:00 UTC against the default branch to catch newly disclosed vulnerabilities promptly, and can be triggered manually from the Actions tab.

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Instead, please report them to the maintainers privately:
- Contact via email: security@tollcraft.org
- Or via Telegram: [Tollcraft](https://t.me/+Gflo5jZStw1jMjE0)

You should receive a response within 48 hours. If the issue is confirmed as a vulnerability, we will open a private security advisory and work on a patch before public disclosure.
