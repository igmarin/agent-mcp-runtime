# Architecture Details

This document provides a comprehensive overview of the design patterns, code organization, and modules that form the `agent-mcp-runtime`.

## Module Directory Structure

The runtime is designed to be highly modular, separating communication channels, agent reasoning engines, and external tool definitions.

- `src/lib.rs`: Registers all library modules.
- `src/runner.rs`: Implements the ReAct (Reasoning and Acting) execution runner.
- `src/providers/mod.rs`: Defines LLM abstractions (`LlmProvider` trait) and mock implementations.
- `src/registry/`: Manages skills and tools.
  - `mod.rs`: Registers registry-level tools.
  - `parser.rs`: Handles markdown frontmatter extraction.
  - `tool.rs`: Defines the `Tool` trait and unit test mock tools.
- `src/mcp/`: Handles Model Context Protocol communication.
  - `mod.rs`: Registers MCP subprocess clients and JSON-RPC types.
  - `jsonrpc.rs`: Declares JSON-RPC 2.0 messages and results.
  - `client.rs`: Manages the stdin/stdout subprocess client connection and the `McpTool` wrapper.

---

## 1. ReAct Execution Loop (`AgentRunner`)

The `AgentRunner` coordinates the reasoning loop. Rather than binding directly to a concrete LLM provider, it operates over the `LlmProvider` trait:

```rust
pub struct AgentRunner<P: LlmProvider> {
    provider: P,
    tools: HashMap<String, Box<dyn Tool>>,
    max_steps: usize,
}
```

### Execution Loop Flow
1. **Compilation**: Combines the user task, descriptions of all registered tools, and formatting instructions into a system prompt.
2. **LLM Query**: Sends the accumulated execution history to the `LlmProvider`.
3. **Parse Step**: Extracts the agent action using `parse_react_step`:
   - If it detects `Final Answer: <message>`, execution terminates and returns the answer.
   - If it detects `Action: <tool>` and `Action Input: <input>`, it routes the call to the corresponding tool.
4. **Execution & Observation**: Invokes the tool asynchronously, formats the result as an `Observation: <result>`, appends it to the prompt history, and loops.
5. **Safety Constraints**: Employs a `max_steps` threshold (defaulting to 5) to prevent infinite billing or execution loops.

---

## 2. Tool Abstraction & The `Tool` Trait

All capabilities exposed to the agent must implement the async `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn call(&self, input: &str) -> Result<String, anyhow::Error>;
}
```

This trait decoupling serves two purposes:
- **Mocking**: Allows creating lightweight mock implementations (`MockTool`) for fast unit tests.
- **MCP Extensibility**: Enables binding remote Model Context Protocol tools directly under the same interface (`McpTool`).

---

## 3. MCP Subprocess Client (`McpClient`)

External tool capabilities are integrated by spawning external servers as subprocesses. Communication relies on JSON-RPC 2.0 protocol payloads over piped standard streams.

```
+---------------+                    stdin (JSON-RPC)                  +------------------+
|               | ===================================================> |                  |
|  McpClient    |                                                      |   MCP Server     |
|               | <=================================================== |   Subprocess     |
+---------------+                    stdout (JSON-RPC)                 +------------------+
```

### Key Design Details:
- **Process Management**: The client spawns and monitors the subprocess using `tokio::process::Command`. The process is kept alive for the duration of the runner lifetime.
- **Standard Stream Piping**: `child.stdin` and `child.stdout` are captured.
- **Mutex Serialization**: An asynchronous `Mutex` protects the stdin/stdout streams. This guarantees that requests are written and responses are read in a strict sequence (JSON-RPC request/response pairing) without interlacing packets.
- **ID Tracking**: The client automatically increments and tracks request IDs to prevent cross-response pollution.

---

## 4. Compile-Time Lints & Code Safety

The runtime implements rigorous quality controls at the compiler level via `Cargo.toml`:

- **Safe Rust**: `unsafe_code = "deny"` prevents the usage of any unsafe blocks, providing strict memory safety guarantees.
- **Strict Clippy**: `clippy::all = "deny"` elevates all common clippy suggestions to compiler errors.
- **Zero Panic Safety**: `clippy::unwrap_used = "warn"` discourages `.unwrap()` usage, forcing developers to implement safe, propagation-based error routing (`Result` and `Option` matching).
- **Thorough Documentation**: `missing_docs = "warn"` requires module and public structure documentation.
