use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPageRequest {
  #[schemars(description = "URL to load in a new page.")]
  pub url: String,
  #[schemars(
    description = "Whether to open the page in the background without bringing it to the front. Default is false (foreground)."
  )]
  pub background: Option<bool>,
  #[schemars(
    description = "If specified, the page is created in an isolated browser context with the given name. Pages in the same browser context share cookies and storage. Pages in different browser contexts are fully isolated (useful for clean-slate testing of cookies and authentication)."
  )]
  pub isolated_context: Option<String>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn new_page(
  Parameters(NewPageRequest {
    url,
    background,
    isolated_context,
    timeout,
  }): Parameters<NewPageRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (url, background, isolated_context, timeout);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
