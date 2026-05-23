//! JSON-RPC 2.0 message definitions for the Model Context Protocol.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 Request message.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be exactly "2.0".
    pub jsonrpc: String,
    /// Numeric or string identifier.
    pub id: i64,
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
    pub id: i64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_jsonrpc_request() -> Result<(), anyhow::Error> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
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
        let result_val = response.result.ok_or_else(|| anyhow::anyhow!("Missing result"))?;
        let tool_result: McpToolCallResult = serde_json::from_value(result_val)?;
        assert_eq!(tool_result.content.len(), 1);
        assert_eq!(tool_result.content[0].content_type, "text");
        assert_eq!(tool_result.content[0].text.as_deref(), Some("Hello world"));
        Ok(())
    }
}
