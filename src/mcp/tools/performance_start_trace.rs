use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceStartTraceRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    default = "reload_default",
    description = "Determines if, once tracing has started, the target page should be automatically reloaded. Navigate the page to the right URL using the navigate_page tool BEFORE starting the trace if reload or autoStop is set to true."
  )]
  pub reload: Option<bool>,
  #[schemars(
    default = "auto_stop_default",
    description = "Determines if the trace recording should be automatically stopped."
  )]
  pub auto_stop: Option<bool>,
  #[schemars(
    description = "The absolute file path, or a file path relative to the current working directory, to save the raw trace data. For example, trace.json.gz (compressed) or trace.json (uncompressed)."
  )]
  pub file_path: Option<String>,
}

fn reload_default() -> Option<bool> {
  Some(true)
}

fn auto_stop_default() -> Option<bool> {
  Some(true)
}

pub async fn performance_start_trace(
  Parameters(PerformanceStartTraceRequest {
    page_id,
    reload,
    auto_stop,
    file_path,
  }): Parameters<PerformanceStartTraceRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, reload, auto_stop, file_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
