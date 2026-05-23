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
    // Check for Final Answer first.
    if let Some(pos) = output.find("Final Answer:") {
        let answer = output[pos + "Final Answer:".len()..].trim().to_string();
        return ReActStep::FinalAnswer(answer);
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
pub struct AgentRunner {
    /// LLM provider reference.
    provider: Box<dyn LlmProvider + Send + Sync>,
    /// Registered tools lookup map.
    tools: HashMap<String, Box<dyn Tool>>,
    /// Maximum step executions permitted.
    max_steps: usize,
}

impl AgentRunner {
    /// Creates a new `AgentRunner` with a provider and max step count.
    #[must_use]
    pub fn new(provider: Box<dyn LlmProvider + Send + Sync>, max_steps: usize) -> Self {
        Self {
            provider,
            tools: HashMap::new(),
            max_steps,
        }
    }

    /// Registers a tool with the runner.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Runs the `ReAct` execution loop for a given task prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM provider requests fail or if the execution loop
    /// exceeds the maximum permitted steps. Tool execution errors are caught, formatted,
    /// and appended as observations to allow the agent to self-correct.
    pub async fn run(&self, task: &str) -> Result<String, anyhow::Error> {
        let mut history =
            "You are a ReAct agent. You solve tasks by executing thoughts and action steps.\n\
             Available tools:\n"
                .to_string();

        use std::fmt::Write as _;
        for tool in self.tools.values() {
            let _ = write!(history, "- {}: {}\n", tool.name(), tool.description());
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

        let _ = write!(history, "Task: {task}\n");

        for step in 1..=self.max_steps {
            println!("\n--- [ReAct Step {step}] ---");
            let response = self.provider.ask_llm(&history).await?;
            println!("{}", response.trim());
            history.push_str(&response);
            history.push('\n');

            match parse_react_step(&response) {
                ReActStep::FinalAnswer(answer) => {
                    return Ok(answer);
                }
                ReActStep::Action { name, input } => {
                    if let Some(tool) = self.tools.get(&name) {
                        println!("-> Call Tool: '{name}' with input: {input}");
                        match tool.call(&input).await {
                            Ok(output) => {
                                println!("<- Observation: {}", output.trim());
                                let observation = format!("Observation: {output}\n");
                                history.push_str(&observation);
                            }
                            Err(e) => {
                                println!("<- Observation Error: {e}");
                                let observation = format!("Observation: Error: {e}\n");
                                history.push_str(&observation);
                            }
                        }
                    } else {
                        println!("<- Observation Error: Tool '{name}' not found");
                        let observation = format!("Observation: Error: Tool '{name}' not found.\n");
                        history.push_str(&observation);
                    }
                }
                ReActStep::ParseError(err) => {
                    println!("<- Observation Error: Parsing failed: {err}");
                    let observation = format!("Observation: Error: Parsing failed: {err}\n");
                    history.push_str(&observation);
                }
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

    #[tokio::test]
    async fn test_agent_runner_loop_success() -> Result<(), anyhow::Error> {
        let mock_provider = StepMockProvider {
            responses: std::sync::Mutex::new(vec![
                "Thought: I need to calculate.\nAction: calc\nAction Input: 2+2".to_string(),
                "Thought: I have the result.\nFinal Answer: The answer is 4".to_string(),
            ]),
        };

        let mut runner = AgentRunner::new(Box::new(mock_provider), 5);
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
