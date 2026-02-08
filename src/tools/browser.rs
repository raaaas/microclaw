use async_trait::async_trait;
use serde_json::json;
use tracing::info;

use crate::claude::ToolDefinition;

use super::{schema_object, Tool, ToolResult};

pub struct BrowserTool;

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser".into(),
            description: "Headless browser automation via agent-browser CLI. State persists across calls.\n\nWorkflow:\n1. `open <url>` — navigate to a URL\n2. `snapshot -i` — get interactive elements with refs (@e1, @e2, ...)\n3. `click @e1` — click an element\n4. `fill @e2 \"text\"` — type into an input\n5. `get text @e3` — extract text content\n6. `screenshot` — capture a visual screenshot\n7. `tabs` — list open tabs\n8. `tab <n>` — switch to tab n\n\nAlways run `snapshot -i` after navigation or interaction to see the updated page state.".into(),
            input_schema: schema_object(
                json!({
                    "command": {
                        "type": "string",
                        "description": "The agent-browser command to run (e.g. `open https://example.com`, `snapshot -i`, `click @e1`)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30)"
                    }
                }),
                &["command"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let command = match input.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'command' parameter".into()),
        };

        let timeout_secs = input
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        info!("Executing browser: {}", command);

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new("agent-browser")
                .arg("--session")
                .arg("microclaw")
                .arg(command)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let mut result_text = String::new();
                if !stdout.is_empty() {
                    result_text.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result_text.is_empty() {
                        result_text.push('\n');
                    }
                    result_text.push_str("STDERR:\n");
                    result_text.push_str(&stderr);
                }
                if result_text.is_empty() {
                    result_text = format!("Command completed with exit code {exit_code}");
                }

                // Truncate very long output
                if result_text.len() > 30000 {
                    result_text.truncate(30000);
                    result_text.push_str("\n... (output truncated)");
                }

                if exit_code == 0 {
                    ToolResult::success(result_text)
                } else {
                    ToolResult::error(format!("Exit code {exit_code}\n{result_text}"))
                }
            }
            Ok(Err(e)) => ToolResult::error(format!("Failed to execute agent-browser: {e}")),
            Err(_) => ToolResult::error(format!(
                "Browser command timed out after {timeout_secs} seconds"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_browser_tool_name_and_definition() {
        let tool = BrowserTool;
        assert_eq!(tool.name(), "browser");
        let def = tool.definition();
        assert_eq!(def.name, "browser");
        assert!(def.description.contains("agent-browser"));
        assert!(def.input_schema["properties"]["command"].is_object());
        assert!(def.input_schema["properties"]["timeout_secs"].is_object());
    }

    #[tokio::test]
    async fn test_browser_missing_command() {
        let tool = BrowserTool;
        let result = tool.execute(json!({})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing 'command'"));
    }
}
