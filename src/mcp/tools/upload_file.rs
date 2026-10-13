use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The uid of the file input element or an element that will open file chooser on the page from the page content snapshot"
  )]
  pub uid: String,
  #[schemars(
    description = "One or more files paths to upload. File paths have to be local to the browser instance (not the MCP)."
  )]
  pub file_paths: Vec<String>,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn upload_file(
  Parameters(UploadFileRequest {
    page_id,
    uid,
    file_paths,
    include_snapshot,
  }): Parameters<UploadFileRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, uid, file_paths, include_snapshot);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
