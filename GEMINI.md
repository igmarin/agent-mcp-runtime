# Gemini Integration Guide (GEMINI.md)

This document provides details on how Google's Gemini API is integrated as the primary, default Large Language Model (LLM) provider in `agent-mcp-runtime`.

---

## 🚀 Getting Started

The Gemini provider is the default option when running the CLI. To use it, you must expose your API key as an environment variable:

```bash
# Set your Gemini API key
export GEMINI_API_KEY="your-api-key-here"

# Run the CLI (uses Gemini and gemini-1.5-flash by default)
agent-mcp-runtime --task "Identify all ruby files in the codebase"
```

If the `GEMINI_API_KEY` is missing or empty, the runtime will report an error and terminate before execution begins.

---

## ⚙️ Configuration Options

The CLI allows you to configure the Gemini provider via the following flags:

| Flag / Option | Description | Default |
|---|---|---|
| `--provider` | Target provider. Specify `gemini`. | `gemini` (Default) |
| `--model` | Target model name. | `gemini-1.5-flash` |
| `--base-url` | Custom API base URL (useful for proxying/testing). | `https://generativelanguage.googleapis.com` |

### Example using a custom model:
To run with `gemini-1.5-pro` for complex reasoning:
```bash
agent-mcp-runtime --provider gemini --model gemini-1.5-pro --task "Implement a custom parser for YARD docs"
```

---

## 🧠 Supported Models

We recommend the following Gemini models based on the task complexity:

1. **`gemini-1.5-flash` (Default)**
   - *Use case*: General developer automation, quick tasks, directory lookups.
   - *Strengths*: Exceptionally fast response times, high token limits, low latency.

2. **`gemini-1.5-pro`**
   - *Use case*: Complex code generation, multi-file refactoring, deep debugging.
   - *Strengths*: Advanced reasoning, high-fidelity code compliance, excellent context retention.

3. **`gemini-2.0-flash` / Newer Models**
   - *Use case*: Sub-second interactive agents, next-gen coding reasoning.
   - *Strengths*: Lower latency and enhanced tool-calling behavior.

---

## 🛠️ Implementation Details

### API Endpoint
The provider targets the v1beta generateContent API:
```
POST {base_url}/v1beta/models/{model}:generateContent?key={api_key}
```

### Request Payload Structure
The provider maps prompts into the standard Gemini JSON request body:
```json
{
  "contents": [
    {
      "parts": [
        {
          "text": "<full_react_prompt_containing_instructions_tools_and_history>"
        }
      ]
    }
  ]
}
```

### Network Policies
- **Timeout**: The HTTP request enforces a strict **30-second connection and read timeout** using the `reqwest` client.
- **Connection Reuse**: The underlying client retains a connection pool to minimize handshake latency during the ReAct loop.
- **Response Validation**: Checks for HTTP status code correctness. Empty response candidate nodes or missing text components return an error, terminating the step safely instead of panicking.
