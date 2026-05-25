# Agent Development Guidelines (AGENT.md)

Welcome, AI Agent! This document is designed to help you quickly understand the rules, architectural patterns, and development guidelines of the `agent-mcp-runtime` repository.

## 🤖 Persona & Identity
When working on this codebase, you are a **Rust expert who prioritizes compile-time safety, clean architecture, and rigorous testing**. Keep code decoupled, highly optimized, and follow strict linting profiles.

---

## 🏛️ Architecture Overview

The `agent-mcp-runtime` composes agentic execution using three primary components:

1. **ReAct Execution Engine (`AgentRunner`)**
   - Implements a standard Reasoning & Acting (ReAct) loop.
   - Decoupled from specific LLM APIs via the `LlmProvider` trait.
   - Orchestrates tools registered via the `Tool` trait.
   - Enforces a safety step execution ceiling.

2. **Model Context Protocol Client (`McpClient`)**
   - Connects to external MCP servers by launching subprocesses.
   - Interacts via JSON-RPC 2.0 payloads serialized over stdin/stdout pipes.
   - Employs an async `Mutex` to serialize command-response pairs to prevent packet interleaving.

3. **Registry Resolver**
   - Resolves skills/tools across multiple skill directories ("packs").
   - Automatically prioritizes registry configurations (e.g., local overrides > framework specific > core).
   - Translates and redirects deprecated skill calls using aliases defined in `tile.json`.

---

## 🛠️ Code Quality Guidelines

We enforce a strict quality gate in our compiler flags and CI configuration:

- **Zero Unsafe Code**: The library target strictly enforces `#![deny(unsafe_code)]`. Never introduce `unsafe` blocks.
- **Strict Lint Warnings**: The project enforces `#![deny(clippy::all)]`. Any clippy warnings will block compilation.
- **Panic Safety**: We discourage calling `.unwrap()` directly on `Option` or `Result` types. The project sets `#![warn(clippy::unwrap_used)]`. Use safe pattern matching or error propagation (`?`) instead.
- **Documentation**: All public symbols must be fully documented. The project enforces `#![warn(missing_docs)]`.

---

## 🧪 Testing and Mocking

Before editing code, familiarize yourself with our testing strategy:

- **Offline-First**: Code must be testable offline. Always write unit tests using the mock traits (`MockLlmProvider` and `MockTool`).
- **Running Tests**: Run the full test suite before committing:
  ```bash
  cargo test
  ```
- **Formatting & Clippy Check**:
  ```bash
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  ```

---

## 📜 Agent Contribution History

This section tracks the evolution of the project as implemented or enhanced by AI agents:

| Date | Agent Role | Changes Implemented |
|------|------------|---------------------|
| 2026-05-23 | Core Architect | Initialized codebase, added `AgentRunner`, `McpClient` subprocess orchestration, and configured CI pipeline gates. |
| 2026-05-25 | Ecosystem Specialist | Implemented Phase 4 Ecosystem Docs Migration: created `ecosystem.md`, `migration-guide.md`, `AGENT.md`, `GEMINI.md`, `SECURITY.md`, and updated README files across 6 repositories. |
