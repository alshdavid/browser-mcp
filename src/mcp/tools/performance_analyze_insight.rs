use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use super::performance_trace;
use crate::mcp::tool_context::RecordedTrace;
use crate::mcp::tool_context::ToolContext;
use crate::mcp::tool_context::TraceInsightSet;

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
  let _ = tool_context.require_page(page_id).await?;

  let Some(trace) = tool_context.last_recorded_trace().await else {
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      "No recorded traces found. Record a performance trace so you have Insights to analyze.",
    )]));
  };

  if trace.insights.is_empty() {
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      "No Performance insights are available for this trace.",
    )]));
  }

  let Some(set) = performance_trace::insight_set(&trace, &insight_set_id) else {
    return Ok(CallToolResult::success(
      describe_missing_insight_set(&trace, &insight_set_id)
        .into_iter()
        .map(ContentBlock::text)
        .collect(),
    ));
  };

  let Some(insight) = set.insights.get(&insight_name).or_else(|| {
    set
      .insights
      .values()
      .find(|insight| insight.name == insight_name)
  }) else {
    return Ok(CallToolResult::success(
      describe_missing_insight(&set, &insight_name)
        .into_iter()
        .map(ContentBlock::text)
        .collect(),
    ));
  };

  Ok(CallToolResult::success(
    vec![
      format!(
        "Insight \"{}\" for insight set \"{}\":",
        insight.name, set.insight_set_id
      ),
      performance_trace::format_insight(insight),
    ]
    .into_iter()
    .map(ContentBlock::text)
    .collect(),
  ))
}

fn describe_missing_insight_set(
  trace: &RecordedTrace,
  insight_set_id: &str,
) -> Vec<String> {
  let mut lines = vec![format!(
    "No Performance Insights for the given insight set id \"{insight_set_id}\". Only use ids given in the \"Available insight sets\" list."
  )];

  lines.push("Available insight sets:".to_owned());
  for set in &trace.insights {
    lines.push(format!("  {} ({})", set.insight_set_id, set.url));
  }

  lines
}

fn describe_missing_insight(
  set: &TraceInsightSet,
  insight_name: &str,
) -> Vec<String> {
  let mut lines = vec![format!(
    "No Insight with the name {insight_name} found. Double check the name you provided is accurate and try again."
  )];

  let mut names = set
    .insights
    .values()
    .map(|insight| insight.name.as_str())
    .collect::<Vec<_>>();
  names.sort();

  lines.push(format!(
    "Available insights in insight set \"{}\": {}",
    set.insight_set_id,
    names.join(", ")
  ));

  lines
}
