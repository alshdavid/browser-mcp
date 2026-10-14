use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use super::performance_start_trace;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceStopTraceRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The absolute file path, or a file path relative to the current working directory, to save the raw trace data. For example, trace.json.gz (compressed) or trace.json (uncompressed)."
  )]
  pub file_path: Option<String>,
}

pub async fn performance_stop_trace(
  tool_context: ToolContext,
  Parameters(PerformanceStopTraceRequest { page_id, file_path }): Parameters<
    PerformanceStopTraceRequest,
  >,
) -> Result<CallToolResult, ErrorData> {
  if !tool_context.is_running_performance_trace().await {
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      "No performance trace is currently running.",
    )]));
  }

  let page = tool_context.require_page(page_id).await?;

  let lines =
    performance_start_trace::stop_and_record(&tool_context, &page, file_path.as_deref()).await?;

  Ok(CallToolResult::success(
    lines.into_iter().map(ContentBlock::text).collect(),
  ))
}
