use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCssStylesRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The uid of the element on the page from the page content snapshot to inspect CSS styles for"
  )]
  pub uid: String,
  #[schemars(
    default = "page_size_default",
    description = "Maximum number of CSS rules to return per page. Defaults to 10."
  )]
  pub page_size: Option<i64>,
  #[schemars(
    default = "page_idx_default",
    description = "Page number to return (0-based). Defaults to 0 (the first page)."
  )]
  pub page_idx: Option<i64>,
}

fn page_size_default() -> Option<i64> {
  Some(10)
}

fn page_idx_default() -> Option<i64> {
  Some(0)
}

pub async fn get_css_styles(
  Parameters(GetCssStylesRequest {
    page_id,
    uid,
    page_size,
    page_idx,
  }): Parameters<GetCssStylesRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, uid, page_size, page_idx);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
