use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::accessibility::AxNode;
use chromiumoxide::cdp::browser_protocol::accessibility::AxValue;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::take_snapshot::accessibility_tree;
use crate::mcp::tools::take_snapshot::assign_uids;
use crate::mcp::tools::take_snapshot::format_tree;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WaitForRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Non-empty list of texts. Resolves when any value appears on the page."
  )]
  pub text: Vec<String>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn wait_for(
  tool_context: ToolContext,
  Parameters(WaitForRequest {
    page_id,
    text,
    timeout,
  }): Parameters<WaitForRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  let text: Vec<String> = text.into_iter().filter(|value| !value.is_empty()).collect();
  if text.is_empty() {
    return Err(ErrorData::invalid_params(
      "At least one non-empty text is required.",
      None,
    ));
  }

  let timeout = timeout
    .filter(|millis| *millis > 0)
    .map_or(DEFAULT_TIMEOUT, |millis| {
      Duration::from_millis(millis as u64)
    });
  let deadline = tokio::time::Instant::now() + timeout;

  loop {
    let nodes = accessibility_tree(&page)
      .await
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

    if find_match(&nodes, &text) {
      let (snapshot_id, previous_uids) = tool_context.begin_snapshot(page_id).await;
      let (root, snapshot) = assign_uids(&nodes, snapshot_id, false, previous_uids);
      tool_context.store_snapshot(page_id, snapshot).await;

      return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
        "Element matching one of {} found.\n## Latest page snapshot\n{}",
        serde_json::Value::from(text),
        format_tree(root.as_ref(), false)
      ))]));
    }

    if tokio::time::Instant::now() >= deadline {
      return Err(ErrorData::internal_error(
        format!(
          "Timed out after {}ms waiting for text {} to appear on the page.",
          timeout.as_millis(),
          serde_json::Value::from(text)
        ),
        None,
      ));
    }

    let next = (tokio::time::Instant::now() + POLL_INTERVAL).min(deadline);
    tokio::time::sleep_until(next).await;
  }
}

fn find_match(
  nodes: &[AxNode],
  text: &[String],
) -> bool {
  nodes.iter().filter(|node| !node.ignored).any(|node| {
    let name = text_of(node.name.as_ref());
    let value = text_of(node.value.as_ref());
    text.iter().any(|needle| {
      name
        .as_deref()
        .is_some_and(|name| name.contains(needle.as_str()))
        || value
          .as_deref()
          .is_some_and(|value| value.contains(needle.as_str()))
    })
  })
}

fn text_of(value: Option<&AxValue>) -> Option<String> {
  match value?.value.as_ref()? {
    serde_json::Value::String(text) => Some(text.clone()),
    _ => None,
  }
}
