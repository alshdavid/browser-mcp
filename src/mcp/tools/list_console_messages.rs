use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ConsoleMessageType {
  #[serde(rename = "log")]
  Log,
  #[serde(rename = "debug")]
  Debug,
  #[serde(rename = "info")]
  Info,
  #[serde(rename = "error")]
  Error,
  #[serde(rename = "warn")]
  Warn,
  #[serde(rename = "dir")]
  Dir,
  #[serde(rename = "dirxml")]
  Dirxml,
  #[serde(rename = "table")]
  Table,
  #[serde(rename = "trace")]
  Trace,
  #[serde(rename = "clear")]
  Clear,
  #[serde(rename = "startGroup")]
  StartGroup,
  #[serde(rename = "startGroupCollapsed")]
  StartGroupCollapsed,
  #[serde(rename = "endGroup")]
  EndGroup,
  #[serde(rename = "assert")]
  Assert,
  #[serde(rename = "profile")]
  Profile,
  #[serde(rename = "profileEnd")]
  ProfileEnd,
  #[serde(rename = "count")]
  Count,
  #[serde(rename = "timeEnd")]
  TimeEnd,
  #[serde(rename = "verbose")]
  Verbose,
  #[serde(rename = "issue")]
  Issue,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListConsoleMessagesRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Maximum number of messages to return. When omitted, returns all messages."
  )]
  pub page_size: Option<i64>,
  #[schemars(
    description = "Page number to return (0-based). When omitted, returns the first page."
  )]
  pub page_idx: Option<i64>,
  #[schemars(
    description = "Filter messages to only return messages of the specified resource types. When omitted or empty, returns all messages."
  )]
  pub types: Option<Vec<ConsoleMessageType>>,
  #[schemars(
    description = "Set to true to return the preserved messages over the last 3 navigations."
  )]
  pub include_preserved_messages: Option<bool>,
  #[schemars(
    description = "Set to true to include the stack trace for each message when available. Increases the response size."
  )]
  pub include_stack_traces: Option<bool>,
  #[schemars(
    description = "Filter messages to only return messages of the specified service worker."
  )]
  pub service_worker_id: Option<String>,
}

pub async fn list_console_messages(
  Parameters(ListConsoleMessagesRequest {
    page_id,
    page_size,
    page_idx,
    types,
    include_preserved_messages,
    include_stack_traces,
    service_worker_id,
  }): Parameters<ListConsoleMessagesRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (
    page_id,
    page_size,
    page_idx,
    types,
    include_preserved_messages,
    include_stack_traces,
    service_worker_id,
  );
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
