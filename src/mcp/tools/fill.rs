use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The uid of an element on the page from the page content snapshot")]
  pub uid: String,
  #[schemars(
    description = "The value to fill in. \"true\" or \"false\" for checkboxes and toggles, \"true\" for radio buttons."
  )]
  pub value: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn fill(
  Parameters(FillRequest {
    page_id,
    uid,
    value,
    include_snapshot,
  }): Parameters<FillRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, uid, value, include_snapshot);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
