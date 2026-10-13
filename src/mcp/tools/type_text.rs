use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TypeTextRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The text to type")]
  pub text: String,
  #[schemars(
    description = "Optional key to press after typing. E.g., \"Enter\", \"Tab\", \"Escape\""
  )]
  pub submit_key: Option<String>,
}

pub async fn type_text(
  tool_context: ToolContext,
  Parameters(TypeTextRequest {
    page_id,
    text,
    submit_key,
  }): Parameters<TypeTextRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, text, submit_key);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
