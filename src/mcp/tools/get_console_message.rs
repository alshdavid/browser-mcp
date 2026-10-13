use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetConsoleMessageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The msgid of a console message on the page from the listed console messages"
  )]
  pub msgid: f64,
}

pub async fn get_console_message(
  Parameters(GetConsoleMessageRequest { page_id, msgid }): Parameters<GetConsoleMessageRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, msgid);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
