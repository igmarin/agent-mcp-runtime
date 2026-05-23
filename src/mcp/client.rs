//! Client implementation for interacting with external Model Context Protocol (MCP) servers.

use crate::mcp::jsonrpc::{JsonRpcRequest, JsonRpcResponse, McpToolCallResult};
use crate::registry::tool::Tool;
use async_trait::async_trait;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// Active connection state to an MCP server subprocess.
struct McpConnection {
    /// The subprocess child handle.
    _child: Child,
    /// Standard input stream of the subprocess.
    stdin: ChildStdin,
    /// Standard output stream of the subprocess.
    stdout: BufReader<ChildStdout>,
    /// Incremental ID tracking for JSON-RPC requests.
    next_id: i64,
}

/// Client for connecting to and communicating with an MCP server subprocess.
pub struct McpClient {
    /// Mutex protecting the stateful subprocess communication channel.
    connection: Mutex<McpConnection>,
}

impl McpClient {
    /// Spawns a new MCP server subprocess and establishes stdin/stdout piping.
    ///
    /// # Errors
    ///
    /// Returns an error if the subprocess fails to spawn or pipes cannot be opened.
    pub fn start(program: &str, args: &[&str]) -> Result<Self, anyhow::Error> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for MCP server"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for MCP server"))?;

        Ok(Self {
            connection: Mutex::new(McpConnection {
                _child: child,
                stdin,
                stdout: BufReader::new(stdout),
                next_id: 1,
            }),
        })
    }

    /// Invokes a remote tool on the MCP server and returns the text response.
    ///
    /// # Errors
    ///
    /// Returns an error if communication fails, JSON serialization/deserialization fails,
    /// or the remote tool returns an execution error.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<String, anyhow::Error> {
        let mut conn = self.connection.lock().await;
        let id = conn.next_id;
        conn.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": name,
                "arguments": arguments,
            }),
        };

        let mut payload = serde_json::to_string(&request)?;
        payload.push('\n');

        // Write request to server stdin
        conn.stdin.write_all(payload.as_bytes()).await?;
        conn.stdin.flush().await?;

        // Read response from server stdout
        let mut response_line = String::new();
        conn.stdout.read_line(&mut response_line).await?;

        if response_line.is_empty() {
            anyhow::bail!("MCP server closed connection unexpectedly");
        }

        let response: JsonRpcResponse = serde_json::from_str(&response_line)?;
        if response.id != id {
            anyhow::bail!(
                "JSON-RPC response ID mismatch: expected {}, got {}",
                id,
                response.id
            );
        }

        if let Some(err) = response.error {
            anyhow::bail!(
                "MCP server returned error (code {}): {}",
                err.code,
                err.message
            );
        }

        let result_val = response
            .result
            .ok_or_else(|| anyhow::anyhow!("Missing result payload in JSON-RPC response"))?;

        let tool_result: McpToolCallResult = serde_json::from_value(result_val)?;

        if tool_result.is_error {
            let err_msg = tool_result
                .content
                .iter()
                .filter(|c| c.content_type == "text")
                .filter_map(|c| c.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!("MCP tool execution failed: {err_msg}");
        }

        let output = tool_result
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .filter_map(|c| c.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(output)
    }
}

/// A wrapper implementing the `Tool` trait for a remote MCP tool.
pub struct McpTool {
    /// The name of the remote tool.
    name: String,
    /// The description of the remote tool.
    description: String,
    /// Reference to the MCP client that will handle tool invocation.
    client: Arc<McpClient>,
}

impl McpTool {
    /// Creates a new `McpTool` bound to a specific client.
    #[must_use]
    pub fn new(name: String, description: String, client: Arc<McpClient>) -> Self {
        Self {
            name,
            description,
            client,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn call(&self, input: &str) -> Result<String, anyhow::Error> {
        // Parse input as JSON arguments, or wrap in a generic object if it's not a JSON object
        let parsed_args = match serde_json::from_str::<serde_json::Value>(input) {
            Ok(val) => val,
            Err(_) => {
                // If it's not a valid JSON string, wrap it as a string argument
                serde_json::json!({ "input": input })
            }
        };

        self.client.call_tool(&self.name, parsed_args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_with_python_loopback() -> Result<(), anyhow::Error> {
        // Simple Python script that acts as an MCP server. It reads JSON-RPC from stdin,
        // and returns a valid JSON-RPC result containing the input argument.
        let python_code = r#"
import sys, json
try:
    line = sys.stdin.readline()
    if line:
        req = json.loads(line)
        resp = {
            "jsonrpc": "2.0",
            "id": req["id"],
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": f"Echo: {req['params']['arguments']['input']}"
                    }
                ]
            }
        }
        print(json.dumps(resp))
        sys.stdout.flush()
except Exception as e:
    sys.exit(1)
"#;

        // Try spawning python3. If it fails, skip the test gracefully.
        let client = match McpClient::start("python3", &["-c", python_code]) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };

        let tool = McpTool::new(
            "echo_tool".to_string(),
            "Echoes input".to_string(),
            Arc::new(client),
        );

        let result = tool.call("hello").await?;
        assert_eq!(result, "Echo: hello");
        Ok(())
    }
}
