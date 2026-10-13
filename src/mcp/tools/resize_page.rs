use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResizePageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Page width")]
  pub width: f64,
  #[schemars(description = "Page height")]
  pub height: f64,
}

pub async fn resize_page(
  Parameters(ResizePageRequest {
    page_id,
    width,
    height,
  }): Parameters<ResizePageRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, width, height);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
