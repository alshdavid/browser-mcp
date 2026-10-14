use chromiumoxide::Page;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::keys::dispatch;
use crate::mcp::tools::keys::parse_key;
use crate::mcp::tools::take_snapshot::accessibility_tree;
use crate::mcp::tools::take_snapshot::assign_uids;
use crate::mcp::tools::take_snapshot::format_tree;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PressKeyRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "A key or a combination (e.g., \"Enter\", \"Control+A\", \"Control++\", \"Control+Shift+R\"). Modifiers: Control, Shift, Alt, Meta"
  )]
  pub key: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn press_key(
  tool_context: ToolContext,
  Parameters(PressKeyRequest {
    page_id,
    key,
    include_snapshot,
  }): Parameters<PressKeyRequest>,
) -> Result<CallToolResult, ErrorData> {
  let tokens = parse_key(&key).map_err(|error| ErrorData::invalid_params(error, None))?;

  let page = tool_context.require_page(page_id).await?;

  dispatch(&page, &tokens)
    .await
    .map_err(|error| ErrorData::internal_error(error, None))?;

  let mut text = format!("Successfully pressed key: {key}");
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

async fn snapshot(
  tool_context: &ToolContext,
  page_id: f64,
  page: &Page,
) -> Result<String, ErrorData> {
  let nodes = accessibility_tree(page)
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  let (snapshot_id, previous_uids) = tool_context.begin_snapshot(page_id).await;
  let (root, snapshot) = assign_uids(&nodes, snapshot_id, false, previous_uids);
  let text = format_tree(root.as_ref(), false);
  tool_context.store_snapshot(page_id, snapshot).await;

  Ok(text)
}
