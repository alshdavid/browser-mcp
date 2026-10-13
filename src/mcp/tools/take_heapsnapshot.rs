use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakeHeapsnapshotRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "A path to a .heapsnapshot file to save the heapsnapshot to.")]
  pub file_path: String,
}

pub async fn take_heapsnapshot(
  Parameters(TakeHeapsnapshotRequest { page_id, file_path }): Parameters<TakeHeapsnapshotRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, file_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
