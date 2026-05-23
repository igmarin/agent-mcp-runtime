//! JSON-RPC 2.0 message definitions for the Model Context Protocol.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 identifier which can be numeric, a string, or null.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// Numeric identifier.
    Number(i64),
    /// String identifier.
    String(String),
    /// Null identifier.
    Null,
}

impl std::fmt::Display for JsonRpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "\"{s}\""),
            Self::Null => write!(f, "null"),
        }
    }
}

impl PartialEq<i64> for JsonRpcId {
    fn eq(&self, other: &i64) -> bool {
        match self {
            Self::Number(n) => n == other,
            _ => false,
        }
    }
}

impl PartialEq<JsonRpcId> for i64 {
    fn eq(&self, other: &JsonRpcId) -> bool {
        other == self
    }
}

impl From<i64> for JsonRpcId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<String> for JsonRpcId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for JsonRpcId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

/// A JSON-RPC 2.0 Request message.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be exactly "2.0".
    pub jsonrpc: String,
    /// Numeric or string identifier.
    pub id: JsonRpcId,
    /// The method being called.
    pub method: String,
    /// The parameters of the call.
    pub params: serde_json::Value,
}

/// A JSON-RPC 2.0 Error object.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Integer error code.
    pub code: i64,
    /// Short error message.
    pub message: String,
    /// Optional structured data about the error.
    pub data: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 Response message.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Must be exactly "2.0".
    pub jsonrpc: String,
    /// Numeric or string identifier matching the request.
    pub id: JsonRpcId,
    /// Successful result value.
    pub result: Option<serde_json::Value>,
    /// Error object, present if the request failed.
    pub error: Option<JsonRpcError>,
}

/// A content item returned from a Model Context Protocol tool call.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct McpContent {
    /// The type of content, e.g., "text".
    #[serde(rename = "type")]
    pub content_type: String,
    /// The actual text content.
    pub text: Option<String>,
}

/// The result structure of an MCP tool call response.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct McpToolCallResult {
    /// List of content items representing the tool execution output.
    pub content: Vec<McpContent>,
    /// Flag indicating if tool execution encountered an error.
    #[serde(default)]
    pub is_error: bool,
}

/// Detailed information about a tool exposed by an MCP server.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct McpToolInfo {
    /// The unique name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// The input schema (JSON schema) expected by the tool.
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// The result structure of an MCP tools/list response.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct McpToolsListResult {
    /// A list of tools supported by the MCP server.
    pub tools: Vec<McpToolInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_jsonrpc_request() -> Result<(), anyhow::Error> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::Number(1),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "hello",
                "arguments": {
                    "msg": "world"
                }
            }),
        };

        let serialized = serde_json::to_string(&request)?;
        assert!(serialized.contains(r#""jsonrpc":"2.0""#));
        assert!(serialized.contains(r#""method":"tools/call""#));
        Ok(())
    }

    #[test]
    fn test_deserialize_jsonrpc_response_success() -> Result<(), anyhow::Error> {
        let raw = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Hello world"
                    }
                ]
            }
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(raw)?;
        assert_eq!(response.id, 1);
        let result_val = response
            .result
            .ok_or_else(|| anyhow::anyhow!("Missing result"))?;
        let tool_result: McpToolCallResult = serde_json::from_value(result_val)?;
        assert_eq!(tool_result.content.len(), 1);
        assert_eq!(tool_result.content[0].content_type, "text");
        assert_eq!(tool_result.content[0].text.as_deref(), Some("Hello world"));
        Ok(())
    }

    #[test]
    fn test_deserialize_jsonrpc_tools_list() -> Result<(), anyhow::Error> {
        let raw = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    {
                        "name": "calculate",
                        "description": "Calculates math expression",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "expression": {"type": "string"}
                            }
                        }
                    }
                ]
            }
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(raw)?;
        assert_eq!(response.id, 2);
        let result_val = response
            .result
            .ok_or_else(|| anyhow::anyhow!("Missing result"))?;
        let tools_list: McpToolsListResult = serde_json::from_value(result_val)?;
        assert_eq!(tools_list.tools.len(), 1);
        assert_eq!(tools_list.tools[0].name, "calculate");
        assert_eq!(
            tools_list.tools[0].description,
            "Calculates math expression"
        );
        Ok(())
    }
}
