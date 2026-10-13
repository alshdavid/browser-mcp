use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::common::DialogAction;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandleDialogRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Whether to dismiss or accept the dialog")]
  pub action: DialogAction,
  #[schemars(description = "Optional prompt text to enter into the dialog.")]
  pub prompt_text: Option<String>,
}

pub async fn handle_dialog(
  tool_context: ToolContext,
  Parameters(HandleDialogRequest {
    page_id,
    action,
    prompt_text,
  }): Parameters<HandleDialogRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, action, prompt_text);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
