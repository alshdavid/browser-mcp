use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HoverRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The uid of an element on the page from the page content snapshot")]
  pub uid: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn hover(
  tool_context: ToolContext,
  Parameters(HoverRequest {
    page_id,
    uid,
    include_snapshot,
  }): Parameters<HoverRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, uid, include_snapshot);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
