use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateScriptRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "A JavaScript function declaration to be executed by the tool in the target page.\nExample without arguments: `() => document.title` or `async () => await fetch(\"example.com\")`.\nExample with arguments: `(el) => el.innerText`\n"
  )]
  pub function: String,
  #[schemars(description = "An optional list of arguments to pass to the function.")]
  pub args: Option<Vec<String>>,
  #[schemars(
    description = "The absolute or relative path to a file to save the script output to. If omitted, the output is returned inline."
  )]
  pub file_path: Option<String>,
  #[schemars(
    description = "Handle dialogs while execution. \"accept\", \"dismiss\", or string for response of window.prompt. Defaults to accept."
  )]
  pub dialog_action: Option<String>,
  #[schemars(
    description = "Whether to wait for the DOM to settle. Pass false if the script only reads data. Defaults to true."
  )]
  pub wait_for_stable_dom: Option<bool>,
}

pub async fn evaluate_script(
  tool_context: ToolContext,
  Parameters(EvaluateScriptRequest {
    page_id,
    function,
    args,
    file_path,
    dialog_action,
    wait_for_stable_dom,
  }): Parameters<EvaluateScriptRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (
    page_id,
    function,
    args,
    file_path,
    dialog_action,
    wait_for_stable_dom,
  );
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
