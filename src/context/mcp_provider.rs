//! Client for querying context from an external HTTP MCP server.

use crate::context::project_context::ProjectContext;
use crate::mcp::jsonrpc::{JsonRpcRequest, JsonRpcResponse, McpToolCallResult};
use crate::registry::manifest::ContextProviderDefinition;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Client to query context from an HTTP MCP server (like rails-ai-bridge).
#[derive(Debug)]
pub struct McpContextProvider {
    pub(crate) endpoint: reqwest::Url,
    pub(crate) optional: bool,
    pub(crate) tools: Vec<String>,
}

impl McpContextProvider {
    /// Creates a new MCP context provider client.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(endpoint: reqwest::Url, optional: bool, tools: Vec<String>) -> Self {
        Self {
            endpoint,
            optional,
            tools,
        }
    }

    /// Creates a new `McpContextProvider` from a name and its manifest definition.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint URL is invalid.
    pub fn from_definition(
        name: &str,
        def: &ContextProviderDefinition,
    ) -> Result<Self, anyhow::Error> {
        let optional = def.optional.unwrap_or(true);
        let tools = def.tools.clone().unwrap_or_else(|| {
            vec![
                "rails_get_schema".to_string(),
                "rails_get_routes".to_string(),
                "rails_get_controllers".to_string(),
                "rails_get_model_details".to_string(),
                "rails_get_config".to_string(),
                "rails_get_gems".to_string(),
                "rails_get_test_info".to_string(),
            ]
        });

        let mut endpoint_str = def.endpoint.clone();
        if !endpoint_str.ends_with("/mcp") && !endpoint_str.ends_with("/mcp/") {
            if endpoint_str.ends_with('/') {
                endpoint_str.push_str("mcp");
            } else {
                endpoint_str.push_str("/mcp");
            }
        }

        let endpoint = reqwest::Url::parse(&endpoint_str)
            .map_err(|e| anyhow::anyhow!("Invalid endpoint URL '{endpoint_str}': {e}"))?;

        println!("Registered context provider '{name}' (endpoint: {endpoint})");
        Ok(Self::new(endpoint, optional, tools))
    }

    /// Queries the MCP provider for project context.
    ///
    /// # Errors
    ///
    /// Returns an error if the context provider is unreachable and `optional` is false.
    pub async fn query(&self, client: &reqwest::Client) -> Result<ProjectContext, anyhow::Error> {
        let mut context = ProjectContext::default();

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

            match self
                .query_tool(client, &self.endpoint, &headers, tool_name)
                .await
            {
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
        url: &reqwest::Url,
        headers: &HeaderMap,
        tool_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let params = if tool_name.starts_with("rails_") {
            serde_json::json!({
                "name": tool_name,
                "arguments": {
                    "detail": "standard"
                }
            })
        } else {
            serde_json::json!({
                "name": tool_name,
                "arguments": {}
            })
        };

        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: crate::mcp::jsonrpc::JsonRpcId::Number(1),
            method: "tools/call".to_string(),
            params,
        };

        let res = client
            .post(url.clone())
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
