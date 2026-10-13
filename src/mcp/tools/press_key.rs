use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PressKeyRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "A key or a combination (e.g., \"Enter\", \"Control+A\", \"Control++\", \"Control+Shift+R\"). Modifiers: Control, Shift, Alt, Meta"
  )]
  pub key: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn press_key(
  Parameters(PressKeyRequest {
    page_id,
    key,
    include_snapshot,
  }): Parameters<PressKeyRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, key, include_snapshot);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
