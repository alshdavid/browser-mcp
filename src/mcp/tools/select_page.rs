use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectPageRequest {
  #[schemars(
    description = "The ID of the page to select. Call list_pages to get available pages."
  )]
  pub page_id: f64,
  #[schemars(description = "Whether to focus the page and bring it to the top.")]
  pub bring_to_front: Option<bool>,
}

pub async fn select_page(
  Parameters(SelectPageRequest {
    page_id,
    bring_to_front,
  }): Parameters<SelectPageRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, bring_to_front);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
