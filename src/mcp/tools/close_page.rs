use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClosePageRequest {
  #[schemars(description = "The ID of the page to close. Call list_pages to list pages.")]
  pub page_id: f64,
}

pub async fn close_page(
  tool_context: ToolContext,
  Parameters(ClosePageRequest { page_id }): Parameters<ClosePageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = page_id;
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
