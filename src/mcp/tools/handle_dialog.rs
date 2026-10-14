use chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams;
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
  let page = tool_context.require_page(page_id).await?;

  if !tool_context.dialog_open(page_id).await {
    return Err(ErrorData::invalid_params("No open dialog found", None));
  }

  let (accept, message) = match action {
    DialogAction::Accept => (true, "Successfully accepted the dialog"),
    DialogAction::Dismiss => (false, "Successfully dismissed the dialog"),
  };

  let mut params = HandleJavaScriptDialogParams::new(accept);
  params.prompt_text = prompt_text;
  // The dialog may already have been dismissed outside of MCP, so a failed CDP
  // call still means it is no longer blocking; erroring would be a false alarm.
  let _ = page.execute(params).await;

  tool_context.clear_dialog(page_id).await;

  Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
}
