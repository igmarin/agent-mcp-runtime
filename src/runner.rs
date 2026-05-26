//! `ReAct` agent execution runner.
//!
//! This module implements the main loop that alternates between reasoning steps,
//! action/tool execution, and updating context history.

use crate::providers::LlmProvider;
use crate::registry::tool::Tool;
use std::collections::HashMap;

/// Represents the parsed step outcome from a `ReAct` LLM response.
#[derive(Debug, PartialEq, Eq)]
enum ReActStep {
    /// The agent wants to execute a tool.
    Action {
        /// The name of the tool to execute.
        name: String,
        /// The raw string input to pass to the tool.
        input: String,
    },
    /// The agent has reached a final answer.
    FinalAnswer(String),
    /// The agent response could not be parsed into a valid `ReAct` step.
    ParseError(String),
}

/// Helper function to parse LLM text output into a `ReActStep`.
fn parse_react_step(output: &str) -> ReActStep {
    // Check for Final Answer first (must be at start of a line).
    for line in output.lines() {
        if let Some(answer) = line.strip_prefix("Final Answer:") {
            return ReActStep::FinalAnswer(answer.trim().to_string());
        }
    }

    let action_pos = output.find("Action:");
    let input_pos = output.find("Action Input:");

    match (action_pos, input_pos) {
        (Some(a_pos), Some(i_pos)) if a_pos < i_pos => {
            let action_line = &output[a_pos + "Action:".len()..i_pos];
            let action_name = action_line
                .trim()
                .lines()
                .next()
                .map_or_else(String::new, |first_line| first_line.trim().to_string());

            let input_line = &output[i_pos + "Action Input:".len()..];
            let action_input = input_line.trim().to_string();

            if action_name.is_empty() {
                ReActStep::ParseError("Parsed empty action name".to_string())
            } else {
                ReActStep::Action {
                    name: action_name,
                    input: action_input,
                }
            }
        }
        _ => ReActStep::ParseError(
            "Could not find a valid Action/Action Input pair or Final Answer in response"
                .to_string(),
        ),
    }
}

/// `ReAct` agent runner orchestrator.
///
/// Handles registration of tools, formatting of instructions, and execution of the
/// `ReAct` loop (Thoughts -> Actions -> Observations).
pub struct AgentRunner {
    /// LLM provider reference.
    provider: Box<dyn LlmProvider + Send + Sync>,
    /// Registered tools lookup map.
    tools: HashMap<String, Box<dyn Tool>>,
    /// Maximum step executions permitted.
    max_steps: usize,
    /// Enable verbose debug logging.
    verbose: bool,
}

impl AgentRunner {
    /// Creates a new `AgentRunner` with a provider and max step count.
    ///
    /// # Arguments
    ///
    /// * `provider` - The dynamic LLM provider instance to query.
    /// * `max_steps` - The maximum number of `ReAct` loop iterations allowed before failing.
    /// * `verbose` - True to enable verbose debug logging to stdout/tracing.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `AgentRunner`.
    #[must_use]
    pub fn new(
        provider: Box<dyn LlmProvider + Send + Sync>,
        max_steps: usize,
        verbose: bool,
    ) -> Self {
        Self {
            provider,
            tools: HashMap::new(),
            max_steps,
            verbose,
        }
    }

    /// Registers a tool with the runner.
    ///
    /// # Arguments
    ///
    /// * `tool` - A boxed tool implementation to register.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Formats the system instructions prompt listing all available tools.
    ///
    /// # Returns
    ///
    /// Returns the system instruction prompt as a string.
    #[must_use]
    pub fn format_system_prompt(&self) -> String {
        use std::fmt::Write as _;

        let mut history =
            "You are a ReAct agent. You solve tasks by executing thoughts and action steps.\n\
             Available tools:\n"
                .to_string();
        for tool in self.tools.values() {
            let _ = writeln!(history, "- {}: {}", tool.name(), tool.description());
        }

        history.push_str(
            "\nUse the following format for each step:\n\
             Thought: <your reasoning>\n\
             Action: <tool_name>\n\
             Action Input: <tool_input>\n\
             Observation: <result of the tool call>\n\
             ... (repeat until done)\n\
             Final Answer: <your final response to the user>\n\n\
             Begin!\n\n",
        );

        history
    }

    /// Resolves and executes a tool by its name.
    ///
    /// If the tool is registered, it runs the tool and returns the formatted observation.
    /// If the tool is not found, or returns an error, it returns a formatted error observation.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to execute.
    /// * `input` - The input argument string to pass to the tool.
    ///
    /// # Returns
    ///
    /// Returns a formatted `Observation: <outcome>\n` string.
    pub async fn execute_tool(&self, name: &str, input: &str) -> String {
        if let Some(tool) = self.tools.get(name) {
            if self.verbose {
                tracing::debug!("-> Call Tool: '{name}' with input: {input}");
            }
            match tool.call(input).await {
                Ok(output) => {
                    if self.verbose {
                        tracing::debug!("<- Observation: {}", output.trim());
                    }
                    format!("Observation: {output}\n")
                }
                Err(e) => {
                    if self.verbose {
                        tracing::debug!("<- Observation Error: {e}");
                    }
                    format!("Observation: Error: {e}\n")
                }
            }
        } else {
            if self.verbose {
                tracing::debug!("<- Observation Error: Tool '{name}' not found");
            }
            format!("Observation: Error: Tool '{name}' not found.\n")
        }
    }

    /// Executes a single step in the `ReAct` reasoning loop.
    ///
    /// Queries the LLM provider, parses the response, executes any requested action,
    /// and appends the reasoning and results to the execution history.
    ///
    /// # Arguments
    ///
    /// * `history` - The mutable history buffer string representing the context.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(answer))` if a final answer is reached.
    /// Returns `Ok(None)` if the loop should continue.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM provider fails to respond.
    pub async fn execute_step(
        &self,
        history: &mut String,
    ) -> Result<Option<String>, anyhow::Error> {
        let response = self.provider.ask_llm(history).await?;
        if self.verbose {
            tracing::debug!("{}", response.trim());
        }
        history.push_str(&response);
        history.push('\n');

        match parse_react_step(&response) {
            ReActStep::FinalAnswer(answer) => Ok(Some(answer)),
            ReActStep::Action { name, input } => {
                let observation = self.execute_tool(&name, &input).await;
                history.push_str(&observation);
                Ok(None)
            }
            ReActStep::ParseError(err) => {
                if self.verbose {
                    tracing::debug!("<- Observation Error: Parsing failed: {err}");
                }
                let observation = format!("Observation: Error: Parsing failed: {err}\n");
                history.push_str(&observation);
                Ok(None)
            }
        }
    }

    /// Runs the `ReAct` execution loop for a given task prompt.
    ///
    /// # Arguments
    ///
    /// * `task` - The user prompt task description to execute.
    ///
    /// # Returns
    ///
    /// Returns the final answer string from the agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM provider request fails or if the execution loop
    /// exceeds the maximum permitted steps. Tool execution errors are caught, formatted,
    /// and appended as observations to allow the agent to self-correct.
    pub async fn run(&self, task: &str) -> Result<String, anyhow::Error> {
        use std::fmt::Write as _;

        let mut history = self.format_system_prompt();
        let _ = writeln!(history, "Task: {task}");

        for step in 1..=self.max_steps {
            if self.verbose {
                tracing::debug!("\n--- [ReAct Step {step}] ---");
            }
            if let Some(answer) = self.execute_step(&mut history).await? {
                return Ok(answer);
            }
        }

        anyhow::bail!(
            "ReAct execution loop exceeded maximum steps ({})",
            self.max_steps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::tool::MockTool;

    // Define a multi-step Mock Provider for testing.
    struct StepMockProvider {
        responses: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl LlmProvider for StepMockProvider {
        async fn ask_llm(&self, _prompt: &str) -> Result<String, anyhow::Error> {
            let mut guard = self
                .responses
                .lock()
                .map_err(|_| anyhow::anyhow!("Mutex lock error"))?;
            if guard.is_empty() {
                anyhow::bail!("No mock response left");
            }
            Ok(guard.remove(0))
        }
    }

    #[test]
    fn test_parse_react_step_final_answer() {
        let input = "Thought: I have finished.\nFinal Answer: Task complete!";
        assert_eq!(
            parse_react_step(input),
            ReActStep::FinalAnswer("Task complete!".to_string())
        );
    }

    #[test]
    fn test_parse_react_step_action() {
        let input = "Thought: I need to call a tool.\nAction: test-tool\nAction Input: hello world";
        assert_eq!(
            parse_react_step(input),
            ReActStep::Action {
                name: "test-tool".to_string(),
                input: "hello world".to_string()
            }
        );
    }

    #[test]
    fn test_parse_react_step_error() {
        let input = "Thought: I don't know what to do next.";
        assert!(matches!(parse_react_step(input), ReActStep::ParseError(_)));
    }

    #[test]
    fn test_format_system_prompt() {
        let mock_provider = StepMockProvider {
            responses: std::sync::Mutex::new(vec![]),
        };
        let mut runner = AgentRunner::new(Box::new(mock_provider), 5, false);
        runner.register_tool(Box::new(MockTool {
            name: "test-tool".to_string(),
            description: "A test tool description".to_string(),
            response: "done".to_string(),
        }));

        let system_prompt = runner.format_system_prompt();
        assert!(system_prompt.contains("Available tools:"));
        assert!(system_prompt.contains("- test-tool: A test tool description"));
    }

    #[tokio::test]
    async fn test_execute_tool_success() {
        let mock_provider = StepMockProvider {
            responses: std::sync::Mutex::new(vec![]),
        };
        let mut runner = AgentRunner::new(Box::new(mock_provider), 5, false);
        runner.register_tool(Box::new(MockTool {
            name: "test-tool".to_string(),
            description: "A test tool description".to_string(),
            response: "done".to_string(),
        }));

        let obs = runner.execute_tool("test-tool", "hello").await;
        assert_eq!(obs, "Observation: done\n");
    }

    #[tokio::test]
    async fn test_execute_tool_not_found() {
        let mock_provider = StepMockProvider {
            responses: std::sync::Mutex::new(vec![]),
        };
        let runner = AgentRunner::new(Box::new(mock_provider), 5, false);

        let obs = runner.execute_tool("missing", "hello").await;
        assert_eq!(obs, "Observation: Error: Tool 'missing' not found.\n");
    }

    #[tokio::test]
    async fn test_agent_runner_loop_success() -> Result<(), anyhow::Error> {
        let mock_provider = StepMockProvider {
            responses: std::sync::Mutex::new(vec![
                "Thought: I need to calculate.\nAction: calc\nAction Input: 2+2".to_string(),
                "Thought: I have the result.\nFinal Answer: The answer is 4".to_string(),
            ]),
        };

        let mut runner = AgentRunner::new(Box::new(mock_provider), 5, false);
        runner.register_tool(Box::new(MockTool {
            name: "calc".to_string(),
            description: "Calculator tool".to_string(),
            response: "4".to_string(),
        }));

        let answer = runner.run("Calculate 2+2").await?;
        assert_eq!(answer, "The answer is 4");
        Ok(())
    }
}
