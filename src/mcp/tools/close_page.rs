use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

pub const CLOSE_PAGE_ERROR: &str = "The last open page cannot be closed";

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
  let Some(page) = tool_context.page(page_id).await else {
    return Err(ErrorData::invalid_params(
      format!("No page found with id {page_id}"),
      None,
    ));
  };

  if tool_context.page_count().await <= 1 {
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      CLOSE_PAGE_ERROR,
    )]));
  }

  let target_id = page.target_id().clone();

  if let Err(error) = page.close().await {
    return Err(ErrorData::internal_error(error.to_string(), None));
  }

  tool_context.drop_page_by_target(&target_id).await;

  Ok(CallToolResult::success(vec![ContentBlock::text(format!(
    "Closed page {page_id}"
  ))]))
}
