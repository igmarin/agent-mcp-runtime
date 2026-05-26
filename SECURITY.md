# Security Policy (SECURITY.md)

We take the security of this project seriously. This document outlines our security policies, supported versions, and best practices to ensure secure operations.

---

## 🛡️ Supported Versions

Only the current active release branches receive security updates:

| Version | Supported | Notes                                     |
|---------|-----------|-------------------------------------------|
| v0.2.x  | Yes       | Current development/stable release.       |
| v0.1.x  | No        | Legacy release. Please upgrade to v0.2.x. |

---

## 🔒 Security Practices in the Codebase

To prevent vulnerabilities, this project enforces strict security controls:

### 1. Memory Safety (Safe Rust)

We strictly forbid the use of unsafe blocks. The project is configured with:

```rust
#![deny(unsafe_code)]
```

This guarantees compile-time memory safety.

### 2. Dependency Auditing
Our CI pipeline runs automated checks using [`rustsec/audit-check`](https://github.com/rustsec/audit-check) on every pull request and push. This validates our dependency tree against the Rust Sec Advisory Database to prevent importing crates with known CVEs.

### 3. Credential Hygiene
**Never commit or hardcode API keys or credentials.**
All providers (Gemini, Claude, Groq, OpenAI) must load their API credentials via standard environment variables (`GEMINI_API_KEY`, `CLAUDE_API_KEY`, etc.). The runtime validates that these keys are not stored or logged.

### 4. Sandboxed Subprocess Communication

External Model Context Protocol (MCP) servers are executed as independent child processes. Communication is limited to JSON-RPC 2.0 messages passing exclusively through piped standard input and output streams. The client does not expose shell command executions or write permission escalations beyond what is defined in the registered tools.

---

## 📞 Reporting a Vulnerability

If you identify a security vulnerability, please you may report it publicly or privately to the repository maintainer.

- **Contact Email**: `ismael.marin@gmail.com`
- **Encryption**: When reporting highly sensitive bugs, please encrypt the payload or request a secure communication channel.

We aim to acknowledge receipt of all security reports within **48 hours** and provide a resolution timeline within **7 days**.
