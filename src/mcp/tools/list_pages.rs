use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPagesRequest {}

pub async fn list_pages(
  tool_context: ToolContext,
  Parameters(ListPagesRequest {}): Parameters<ListPagesRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
