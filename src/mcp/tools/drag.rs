use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DragRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[serde(rename = "from_uid")]
  #[schemars(description = "The uid of the element to drag")]
  pub from_uid: String,
  #[serde(rename = "to_uid")]
  #[schemars(description = "The uid of the element to drop into")]
  pub to_uid: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn drag(
  tool_context: ToolContext,
  Parameters(DragRequest {
    page_id,
    from_uid,
    to_uid,
    include_snapshot,
  }): Parameters<DragRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, from_uid, to_uid, include_snapshot);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
