use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakeSnapshotRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Whether to include all possible information available in the full a11y tree. Default is false."
  )]
  pub verbose: Option<bool>,
  #[schemars(
    description = "The absolute path, or a path relative to the current working directory, to save the snapshot to instead of attaching it to the response."
  )]
  pub file_path: Option<String>,
}

pub async fn take_snapshot(
  Parameters(TakeSnapshotRequest {
    page_id,
    verbose,
    file_path,
  }): Parameters<TakeSnapshotRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, verbose, file_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
