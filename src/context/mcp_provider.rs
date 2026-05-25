//! Client for querying context from an external HTTP MCP server.

use crate::context::project_context::ProjectContext;
use crate::mcp::jsonrpc::{JsonRpcRequest, JsonRpcResponse, McpToolCallResult};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Client to query context from an HTTP MCP server (like rails-ai-bridge).
#[derive(Debug)]
pub struct McpContextProvider {
    pub(crate) endpoint: String,
    pub(crate) optional: bool,
    pub(crate) tools: Vec<String>,
}

impl McpContextProvider {
    /// Creates a new MCP context provider client.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(endpoint: String, optional: bool, tools: Vec<String>) -> Self {
        Self {
            endpoint,
            optional,
            tools,
        }
    }

    /// Queries the MCP provider for project context.
    ///
    /// # Errors
    ///
    /// Returns an error if the context provider is unreachable and `optional` is false.
    pub async fn query(&self) -> Result<ProjectContext, anyhow::Error> {
        let mut context = ProjectContext::default();
        let client = reqwest::Client::new();

        // Target URL
        let mut url = self.endpoint.clone();
        if !url.ends_with("/mcp") && !url.ends_with("/mcp/") {
            if url.ends_with('/') {
                url.push_str("mcp");
            } else {
                url.push_str("/mcp");
            }
        }

        // Look up RAILS_AI_BRIDGE_MCP_TOKEN or RAILS_AI_CONTEXT_TOKEN env var for auth
        let token = std::env::var("RAILS_AI_BRIDGE_MCP_TOKEN")
            .ok()
            .or_else(|| std::env::var("RAILS_AI_CONTEXT_TOKEN").ok());

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(t) = token {
            let auth_val = format!("Bearer {}", t.trim());
            if let Ok(val) = HeaderValue::from_str(&auth_val) {
                headers.insert(AUTHORIZATION, val);
            }
        }

        for tool_name in &self.tools {
            println!("Querying context provider tool: {tool_name}...");

            match self.query_tool(&client, &url, &headers, tool_name).await {
                Ok(Some(text_content)) => match tool_name.as_str() {
                    "rails_get_schema" => context.schema = Some(text_content),
                    "rails_get_routes" => context.routes = Some(text_content),
                    "rails_get_controllers" => context.controllers = Some(text_content),
                    "rails_get_model_details" => context.models = Some(text_content),
                    "rails_get_config" => context.config = Some(text_content),
                    "rails_get_gems" => context.gems = Some(text_content),
                    "rails_get_test_info" => context.tests = Some(text_content),
                    _ => {}
                },
                Ok(None) => {}
                Err(e) => {
                    if self.optional {
                        println!("Warning: context provider tool '{tool_name}' failed: {e}");
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(context)
    }

    /// Queries a single tool from the context provider.
    async fn query_tool(
        &self,
        client: &reqwest::Client,
        url: &str,
        headers: &HeaderMap,
        tool_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let params = match tool_name {
            "rails_get_schema"
            | "rails_get_routes"
            | "rails_get_controllers"
            | "rails_get_model_details"
            | "rails_get_config"
            | "rails_get_gems"
            | "rails_get_test_info" => {
                serde_json::json!({
                    "name": tool_name,
                    "arguments": {
                        "detail": "standard"
                    }
                })
            }
            _ => {
                serde_json::json!({
                    "name": tool_name,
                    "arguments": {}
                })
            }
        };

        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: crate::mcp::jsonrpc::JsonRpcId::Number(1),
            method: "tools/call".to_string(),
            params,
        };

        let res = client
            .post(url)
            .headers(headers.clone())
            .json(&rpc_request)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            anyhow::bail!("Context provider request failed with status: {status}");
        }

        let rpc_response: JsonRpcResponse = res.json().await?;

        if let Some(err) = rpc_response.error {
            let err_msg = &err.message;
            anyhow::bail!("Tool '{tool_name}' returned error: {err_msg}");
        }

        if let Some(result_value) = rpc_response.result {
            let call_result: McpToolCallResult = serde_json::from_value(result_value)?;
            let text_content = call_result
                .content
                .iter()
                .filter_map(|c| c.text.clone())
                .collect::<Vec<String>>()
                .join("\n");
            return Ok(Some(text_content));
        }

        Ok(None)
    }
}
