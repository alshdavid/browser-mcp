use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::DescribeNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::ScrollIntoViewIfNeededParams;
use chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventParams;
use chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType;
use chromiumoxide::cdp::browser_protocol::input::MouseButton;
use chromiumoxide::layout::Point;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::click::clickable_point;
use crate::mcp::tools::fill::snapshot;
use crate::mcp::tools::fill::unknown_uid;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const DROP_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DragRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[serde(rename = "from_uid")]
  #[schemars(description = "The uid of the element to drag")]
  pub from_uid: String,
  #[serde(rename = "to_uid")]
  #[schemars(description = "The uid of the element to drop into")]
  pub to_uid: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn drag(
  tool_context: ToolContext,
  Parameters(DragRequest {
    page_id,
    from_uid,
    to_uid,
    include_snapshot,
  }): Parameters<DragRequest>,
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

  let from_node_id = tool_context
    .backend_node_for_uid(page_id, &from_uid)
    .await
    .ok_or_else(|| unknown_uid(&from_uid, page_id))?;

  let to_node_id = tool_context
    .backend_node_for_uid(page_id, &to_uid)
    .await
    .ok_or_else(|| unknown_uid(&to_uid, page_id))?;

  let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT;

  drag_element(&page, from_node_id, to_node_id, deadline, &from_uid).await?;

  let mut text = "Successfully dragged an element".to_owned();
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

async fn drag_element(
  page: &Page,
  from_node_id: i64,
  to_node_id: i64,
  deadline: tokio::time::Instant,
  uid: &str,
) -> Result<(), ErrorData> {
  loop {
    match attempt(page, from_node_id, to_node_id).await {
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
  from_node_id: i64,
  to_node_id: i64,
) -> Result<(), Failure> {
  let from = BackendNodeId::new(from_node_id);
  let to = BackendNodeId::new(to_node_id);

  let from_point = visible_point(page, from).await?;
  let to_point = visible_point(page, to).await?;

  dispatch_mouse(
    page,
    DispatchMouseEventType::MouseMoved,
    from_point,
    MouseButton::None,
    0,
  )
  .await
  .map_err(|error| Failure::fatal(error.to_string()))?;

  dispatch_mouse(
    page,
    DispatchMouseEventType::MousePressed,
    from_point,
    MouseButton::Left,
    1,
  )
  .await
  .map_err(|error| Failure::fatal(error.to_string()))?;

  let outcome = async {
    dispatch_mouse(
      page,
      DispatchMouseEventType::MouseMoved,
      to_point,
      MouseButton::None,
      1,
    )
    .await?;

    tokio::time::sleep(DROP_DELAY).await;

    dispatch_mouse(
      page,
      DispatchMouseEventType::MouseMoved,
      to_point,
      MouseButton::None,
      1,
    )
    .await?;

    Ok::<(), chromiumoxide::error::CdpError>(())
  }
  .await;

  // The button is already down: without a release it stays pressed for the rest
  // of the session, and that cleanup error must not replace the real failure.
  let released = dispatch_mouse(
    page,
    DispatchMouseEventType::MouseReleased,
    to_point,
    MouseButton::Left,
    0,
  )
  .await;

  match outcome {
    Ok(()) => released.map_err(|error| Failure::fatal(error.to_string())),
    Err(error) => Err(Failure::fatal(error.to_string())),
  }
}

async fn visible_point(
  page: &Page,
  backend_node_id: BackendNodeId,
) -> Result<Point, Failure> {
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

  clickable_point(page, backend_node_id)
    .await
    .map_err(Failure::retry)
}

pub(crate) async fn dispatch_mouse(
  page: &Page,
  event_type: DispatchMouseEventType,
  point: Point,
  button: MouseButton,
  buttons: i64,
) -> Result<(), chromiumoxide::error::CdpError> {
  page
    .execute(
      DispatchMouseEventParams::builder()
        .r#type(event_type)
        .x(point.x)
        .y(point.y)
        .button(button)
        .buttons(buttons)
        .click_count(1)
        .build()
        .expect("mouse event parameters are always valid"),
    )
    .await?;

  Ok(())
}
