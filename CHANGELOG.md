# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-23

### Added
- **ReAct Execution Engine**: Built the async `AgentRunner` reasoning engine, supporting step limit thresholds, and real-time execution logging to the terminal.
- **Model Context Protocol (MCP) Integration**: Implemented a long-running subprocess spawner (`McpClient`) managing stdin/stdout pipes, JSON-RPC 2.0 frames, and automatic process termination (`kill_on_drop`).
- **Dynamic Tool Discovery**: Supported standard `tools/list` protocol queries, allowing the runtime to list and wrap remote server capabilities into the `Tool` trait automatically.
- **Diverse LLM Provider Ecosystem**: Integrated asynchronous clients for:
  - **Google Gemini** (via AI Developer API)
  - **OpenAI** (compatible out-of-the-box with **OpenRouter**, **Ollama**, and custom Cursor/Windsurf endpoints)
  - **Anthropic Claude** (via messages API)
  - **Groq** (supporting low-latency Llama3 reasoning loops)
- **TDD Frontmatter Parser**: Created a line-based skill parser in `registry::parser` for safely extracting skill details and configurations from markdown headers.
- **CLI Binary Interface**: Built `src/main.rs` featuring a `clap`-based command line parser to mount MCP servers, configure parameters, and choose LLM engines.
- **Strict Lint Configurations**: Configured deny rules for unsafe code and compiler errors for common clippy lints.
- **Hardened GitHub Actions CI**: Established formatting checks, clippy validation, test suite runs, and automated dependency security scans using `rustsec/audit-check`.
