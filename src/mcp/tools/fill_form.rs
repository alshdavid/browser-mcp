use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::fill::DEFAULT_TIMEOUT;
use crate::mcp::tools::fill::TIMEOUT_PER_CHAR;
use crate::mcp::tools::fill::fill_element;
use crate::mcp::tools::fill::has_option_children;
use crate::mcp::tools::fill::snapshot;
use crate::mcp::tools::fill::unknown_uid;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillFormElement {
  #[schemars(description = "The uid of the element to fill out")]
  pub uid: String,
  #[schemars(
    description = "Value for the element. \"true\" or \"false\" for checkboxes and toggles, \"true\" for radio buttons."
  )]
  pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillFormRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Elements from snapshot to fill out.")]
  pub elements: Vec<FillFormElement>,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn fill_form(
  tool_context: ToolContext,
  Parameters(FillFormRequest {
    page_id,
    elements,
    include_snapshot,
  }): Parameters<FillFormRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context.require_page(page_id).await?;

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  for FillFormElement { uid, value } in &elements {
    let node = tool_context
      .node_for_uid(page_id, uid)
      .await
      .ok_or_else(|| unknown_uid(uid, page_id))?;

    let backend_node_id = node
      .backend_node_id
      .filter(|backend_node_id| *backend_node_id != 0)
      .ok_or_else(|| unknown_uid(uid, page_id))?;

    let is_combobox_select =
      node.role == "combobox" && has_option_children(&page, backend_node_id).await?;

    let length = u32::try_from(value.chars().count()).unwrap_or(u32::MAX);
    let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT + TIMEOUT_PER_CHAR * length;

    fill_element(
      &page,
      backend_node_id,
      value,
      deadline,
      uid,
      is_combobox_select,
    )
    .await?;
  }

  let mut text = "Successfully filled out the form".to_owned();
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}
