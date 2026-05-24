# Agent MCP Runtime

![Agent MCP Runtime Logo](https://github.com/user-attachments/assets/2e417a58-732e-4934-977f-7e4731dc33b7)

**Note**: This project is currently under active development and may undergo significant changes.

> ![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)
> ![Rust](https://img.shields.io/badge/rust-1.74%2B-blue)
> ![Gemini](https://img.shields.io/badge/gemini-1.5%2B-blue)
> ![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/igmarin/agent-mcp-runtime?utm_source=oss&utm_medium=github&utm_campaign=igmarin%2Fagent-mcp-runtime&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)

## Key Features

- **Strict Compile-Time Safety**: Zero unsafe code permitted (`unsafe_code = "deny"`) and strict workspace linting gates.
- **Asynchronous ReAct Runner**: Orchestrates reasoning and action loops using customizable LLM providers.
- **Model Context Protocol (MCP) Client**: Integrates external tools by spawning long-running subprocesses and exchanging JSON-RPC 2.0 messages over stdout/stdin.
- **TDD Frontmatter Parser**: Parses Markdown frontmatter to extract metadata for agent skills/tools.
- **Test-Driven Design & Mocking**: Includes clean trait abstractions for LLM providers (`LlmProvider`) and tools (`Tool`), featuring mock implementations for fully offline, fast testing.
- **GitHub Actions CI/CD**: Automatic code formatting, strict clippy checks, test suites, and vulnerability scanning (`rustsec/audit-check`).

## Architecture

```mermaid
graph TD
    User([Task Request]) --> Runner[AgentRunner]
    Runner --> Parser[FrontmatterParser]
    Runner --> Provider[LlmProvider Trait]
    Runner --> Tool[Tool Trait]
    Provider --> MockProvider[MockLlmProvider]
    Tool --> MockTool[MockTool]
    Tool --> McpTool[McpTool]
    McpTool --> Client[McpClient]
    Client --> Subprocess[MCP Server Subprocess]
```

For a detailed walkthrough of the system architecture and components, see [docs/architecture.md](docs/architecture.md).

## Getting Started

### Prerequisites

Ensure you have Rust (stable 1.74+) installed.

### Building and Testing

Check out [docs/getting_started.md](docs/getting_started.md) for more details.

1. **Verify Formatting**

   ```bash
   cargo fmt --check
   ```

2. **Run Lints & Clippy**

   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

3. **Run Test Suite**

   ```bash
   cargo test
   ```

## Repository Health

We practice strict repository hygiene. Every commit is audited in GitHub Actions CI for formatting, clippy warnings, tests correctness, and dependency security audits via Cargo Audit.
