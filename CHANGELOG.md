# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Optional `context_providers` configuration in `registry.json` manifest.
- Unified project context module (`src/context/`) supporting HTTP MCP clients (like `rails-ai-bridge`).
- Forwarding of bearer authentication token via `RAILS_AI_BRIDGE_MCP_TOKEN` in context provider queries.
- Exposed `get_project_context` MCP tool to allow agents to query unified project schemas, routes, models, and dependencies.
- Refactored `ContextProviderRegistry::from_manifest` to delegate to a clean factory method `McpContextProvider::from_definition`, simplifying the functional iterator pipeline.
- Applied Rust best practices: structured destructuring in `ProjectContext::merge`, typed `reqwest::Url` parsing on registration, `reqwest::Client` connection pool reuse, and DRY dynamic tool prefix matching.
- Resolved CodeRabbit review findings: handled `McpToolCallResult::is_error` flag inside `query_tool`, added 30-second client timeouts, enforced non-optional provider errors inside `query_all` (propagated via `main.rs`), and sorted providers alphabetically by name for deterministic merge orders.

### Changed
- Refactored CLI bootstrapper in `main.rs` to delegate concerns to separate, SRP-compliant services.
- Extracted `LlmProviderFactory` service to dynamically instantiate and validate LLM providers.
- Extracted `PackResolverService` service to handle auto-detection, pack loading, and manifest resolution.
- Extracted `GitRunner` trait from `SkillSourceResolver` to make caching operations mockable, adding automatic checkout directory cleanup.
- Refactored `AgentRunner` execution methods to simplify loops and add structured, YARD-equivalent documentation.
- Decoupled `McpContextProvider` and `ProjectContext` from Ruby/Rails tool hardcodings, adding dynamic mapping configurations via `ContextToolSpec`.
- Refactored JSON-RPC stream communication in `McpClient` into helper methods on `McpConnection`.

## [0.2.0] - 2026-05-25

### Added
- **Ecosystem Overview Documentation**: Added `docs/ecosystem.md` detailing the multi-repository AI skill architecture, package resolution logic, and future integration points.
- **Migration Guide**: Created `docs/migration-guide.md` tracking the relocation of 12 skills to `ruby-core-skills` and outlining the update path for downstream projects.
- **Agent Development Manual (`AGENT.md`)**: Added compiler-level guardrails, trait patterns, and guidelines to coordinate future AI agent developments.
- **Gemini Integration Guide (`GEMINI.md`)**: Documented the configuration, model tiers, and JSON request/response payloads for Google Gemini.
- **Security Policy (`SECURITY.md`)**: Configured reporting steps, compile-time memory safety mandates, and secret sanitation routines.

### Changed
- **Unified Ecosystem READMEs**: Updated READMEs across all 6 core repositories to include a standard "Part of the AI Skill Ecosystem" navigation table.

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
