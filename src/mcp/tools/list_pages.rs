use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPagesRequest {}

pub async fn list_pages(
  Parameters(ListPagesRequest {}): Parameters<ListPagesRequest>
) -> Result<CallToolResult, ErrorData> {
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
