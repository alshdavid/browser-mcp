use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WaitForRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Non-empty list of texts. Resolves when any value appears on the page."
  )]
  pub text: Vec<String>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn wait_for(
  Parameters(WaitForRequest {
    page_id,
    text,
    timeout,
  }): Parameters<WaitForRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, text, timeout);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
