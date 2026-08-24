# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x | ✅ |

## Reporting a Vulnerability

Please report security issues privately — **do not** open a public issue.

- Email: **lnikqi17@gmail.com**
- Or: GitHub private vulnerability report (Security → Report a vulnerability)

Include: description, steps to reproduce, impact, and affected version/commit.

We aim to acknowledge within 48h and release a fix as soon as possible. Once fixed, we will publish a GitHub Security Advisory and release notes.

## App Security Notes

- QueryCastle runs locally (Tauri). No telemetry.
- Connections, tabs, history and saved queries are stored in `localStorage` on your device only.
- Database credentials are stored locally in plain text for convenience — use OS-level disk encryption if needed.
- SSL/TLS optional per connection (`sslmode=require` for Postgres, `native-tls` for MySQL).
- Auto-updates are signed (`minisign`) and verified against `pubkey` in `tauri.conf.json` before install.
- `F5`/refresh does not send data externally; `mainView` and tabs are persisted locally.
