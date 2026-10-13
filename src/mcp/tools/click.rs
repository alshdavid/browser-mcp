use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::DescribeNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::GetContentQuadsParams;
use chromiumoxide::cdp::browser_protocol::dom::ScrollIntoViewIfNeededParams;
use chromiumoxide::layout::ElementQuad;
use chromiumoxide::layout::Point;
use chromiumoxide::types::ClickOptions;
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
pub struct ClickRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The uid of an element on the page from the page content snapshot")]
  pub uid: String,
  #[schemars(description = "Set to true for double clicks. Default is false.")]
  pub dbl_click: Option<bool>,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn click(
  tool_context: ToolContext,
  Parameters(ClickRequest {
    page_id,
    uid,
    dbl_click,
    include_snapshot,
  }): Parameters<ClickRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  let backend_node_id = tool_context
    .backend_node_for_uid(page_id, &uid)
    .await
    .ok_or_else(|| {
      ErrorData::invalid_params(
        format!("Element uid \"{uid}\" not found on page {page_id}."),
        None,
      )
    })?;

  let click_count = if dbl_click.unwrap_or(false) { 2 } else { 1 };
  let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT;

  click_element(&page, backend_node_id, click_count, deadline, &uid).await?;

  let text = if click_count == 2 {
    "Successfully double clicked on the element"
  } else {
    "Successfully clicked on the element"
  };

  let mut text = text.to_owned();
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

async fn click_element(
  page: &Page,
  backend_node_id: i64,
  click_count: i64,
  deadline: tokio::time::Instant,
  uid: &str,
) -> Result<(), ErrorData> {
  loop {
    match attempt(page, backend_node_id, click_count).await {
      Ok(()) => return Ok(()),
      Err(failure) if failure.vanished => {
        return Err(ErrorData::invalid_params(
          format!("Element with uid {uid} no longer exists on the page."),
          None,
        ));
      }
      Err(failure) if failure.retryable && tokio::time::Instant::now() < deadline => {
        let next = (tokio::time::Instant::now() + POLL_INTERVAL).min(deadline);
        tokio::time::sleep_until(next).await;
      }
      Err(failure) => {
        return Err(ErrorData::internal_error(
          format!(
            "Failed to interact with the element with uid {uid}. {}",
            failure.reason
          ),
          None,
        ));
      }
    }
  }
}

struct Failure {
  reason: String,
  retryable: bool,
  vanished: bool,
}

impl Failure {
  fn retry(reason: String) -> Self {
    Self {
      reason,
      retryable: true,
      vanished: false,
    }
  }

  fn fatal(reason: String) -> Self {
    Self {
      reason,
      retryable: false,
      vanished: false,
    }
  }

  fn classify(reason: String) -> Self {
    let vanished = reason.contains("No node with given id found")
      || reason.contains("Could not find node")
      || reason.contains("does not belong to the document");

    Self {
      reason,
      retryable: vanished,
      vanished,
    }
  }
}

async fn attempt(
  page: &Page,
  backend_node_id: i64,
  click_count: i64,
) -> Result<(), Failure> {
  let backend_node_id = BackendNodeId::new(backend_node_id);

  page
    .execute(
      DescribeNodeParams::builder()
        .backend_node_id(backend_node_id)
        .depth(100)
        .build(),
    )
    .await
    .map_err(|error| Failure::classify(error.to_string()))?;

  page
    .execute(
      ScrollIntoViewIfNeededParams::builder()
        .backend_node_id(backend_node_id)
        .build(),
    )
    .await
    .map_err(|error| Failure::classify(error.to_string()))?;

  let point = clickable_point(page, backend_node_id)
    .await
    .map_err(Failure::retry)?;

  page
    .click_with(
      point,
      ClickOptions::builder().click_count(click_count).build(),
    )
    .await
    .map_err(|error| Failure::fatal(error.to_string()))?;

  Ok(())
}

async fn clickable_point(
  page: &Page,
  backend_node_id: BackendNodeId,
) -> Result<Point, String> {
  let quads = page
    .execute(
      GetContentQuadsParams::builder()
        .backend_node_id(backend_node_id)
        .build(),
    )
    .await
    .map_err(|error| error.to_string())?
    .result
    .quads;

  quads
    .iter()
    .filter(|quad| quad.inner().len() == 8)
    .map(ElementQuad::from_quad)
    .filter(|quad| quad.quad_area() > 1.)
    .map(|quad| quad.quad_center())
    .next()
    .ok_or_else(|| "Node is either not visible or not an HTMLElement".to_owned())
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
