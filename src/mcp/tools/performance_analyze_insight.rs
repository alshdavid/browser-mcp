use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceAnalyzeInsightRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The id for the specific insight set. Only use the ids given in the \"Available insight sets\" list."
  )]
  pub insight_set_id: String,
  #[schemars(
    description = "The name of the Insight you want more information on. For example: \"DocumentLatency\" or \"LCPBreakdown\""
  )]
  pub insight_name: String,
}

pub async fn performance_analyze_insight(
  tool_context: ToolContext,
  Parameters(PerformanceAnalyzeInsightRequest {
    page_id,
    insight_set_id,
    insight_name,
  }): Parameters<PerformanceAnalyzeInsightRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, insight_set_id, insight_name);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
