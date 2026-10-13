use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::common::DialogAction;
use crate::mcp::tools::common::NavigationType;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NavigatePageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Navigate the page by URL, back or forward in history, or reload.")]
  pub r#type: Option<NavigationType>,
  #[schemars(description = "Target URL (only type=url)")]
  pub url: Option<String>,
  #[schemars(description = "Whether to ignore cache on reload.")]
  pub ignore_cache: Option<bool>,
  #[schemars(
    description = "Whether to auto accept or beforeunload dialogs triggered by this navigation. Default is accept."
  )]
  pub handle_before_unload: Option<DialogAction>,
  #[schemars(
    description = "A JavaScript script to be executed on each new document before any other scripts for the next navigation."
  )]
  pub init_script: Option<String>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn navigate_page(
  tool_context: ToolContext,
  Parameters(NavigatePageRequest {
    page_id,
    r#type,
    url,
    ignore_cache,
    handle_before_unload,
    init_script,
    timeout,
  }): Parameters<NavigatePageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (
    page_id,
    r#type,
    url,
    ignore_cache,
    handle_before_unload,
    init_script,
    timeout,
  );
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
