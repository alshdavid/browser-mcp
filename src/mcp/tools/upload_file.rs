use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::DescribeNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::ScrollIntoViewIfNeededParams;
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType;
use chromiumoxide::cdp::browser_protocol::input::MouseButton;
use chromiumoxide::cdp::browser_protocol::page::EventFileChooserOpened;
use chromiumoxide::cdp::browser_protocol::page::SetInterceptFileChooserDialogParams;
use futures_util::StreamExt;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::click::clickable_point;
use crate::mcp::tools::drag::dispatch_mouse;
use crate::mcp::tools::fill::snapshot;
use crate::mcp::tools::fill::unknown_uid;

const FILE_CHOOSER_TIMEOUT: Duration = Duration::from_millis(3000);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The uid of the file input element or an element that will open file chooser on the page from the page content snapshot"
  )]
  pub uid: String,
  #[schemars(
    description = "One or more files paths to upload. File paths have to be local to the browser instance (not the MCP)."
  )]
  pub file_paths: Vec<String>,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn upload_file(
  tool_context: ToolContext,
  Parameters(UploadFileRequest {
    page_id,
    uid,
    file_paths,
    include_snapshot,
  }): Parameters<UploadFileRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context.require_page(page_id).await?;

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  let node_id = tool_context
    .backend_node_for_uid(page_id, &uid)
    .await
    .ok_or_else(|| unknown_uid(&uid, page_id))?;

  let backend_node_id = BackendNodeId::new(node_id);

  if set_files(&page, backend_node_id, &file_paths)
    .await
    .is_err()
  {
    upload_through_chooser(&page, backend_node_id, &file_paths).await?;
  }

  let mut text = format!("File uploaded from {}.", file_paths.join(", "));
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

async fn set_files(
  page: &chromiumoxide::Page,
  backend_node_id: BackendNodeId,
  file_paths: &[String],
) -> Result<(), chromiumoxide::error::CdpError> {
  page
    .execute(
      SetFileInputFilesParams::builder()
        .files(file_paths.iter().cloned())
        .backend_node_id(backend_node_id)
        .build()
        .expect("file input parameters are always valid"),
    )
    .await?;

  Ok(())
}

async fn upload_through_chooser(
  page: &chromiumoxide::Page,
  backend_node_id: BackendNodeId,
  file_paths: &[String],
) -> Result<(), ErrorData> {
  // The chooser event only fires while a click is in flight, so the
  // subscription has to exist before the click is dispatched.
  let mut chooser = page
    .event_listener::<EventFileChooserOpened>()
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  page
    .execute(
      SetInterceptFileChooserDialogParams::builder()
        .enabled(true)
        .build()
        .expect("file chooser interception parameters are always valid"),
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  let outcome = async {
    click(page, backend_node_id).await?;

    let opened = tokio::time::timeout(FILE_CHOOSER_TIMEOUT, chooser.next())
      .await
      .map_err(|_| upload_failure())?
      .ok_or_else(upload_failure)?;

    let chooser_node_id = opened.backend_node_id.ok_or_else(upload_failure)?;

    set_files(page, chooser_node_id, file_paths)
      .await
      .map_err(|_| upload_failure())
  }
  .await;

  let disabled = page
    .execute(
      SetInterceptFileChooserDialogParams::builder()
        .enabled(false)
        .build()
        .expect("file chooser interception parameters are always valid"),
    )
    .await;

  outcome?;
  disabled.map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  Ok(())
}

async fn click(
  page: &chromiumoxide::Page,
  backend_node_id: BackendNodeId,
) -> Result<(), ErrorData> {
  page
    .execute(
      DescribeNodeParams::builder()
        .backend_node_id(backend_node_id)
        .depth(100)
        .build(),
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  page
    .execute(
      ScrollIntoViewIfNeededParams::builder()
        .backend_node_id(backend_node_id)
        .build(),
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  let point = clickable_point(page, backend_node_id)
    .await
    .map_err(|error| ErrorData::internal_error(error, None))?;

  dispatch_mouse(
    page,
    DispatchMouseEventType::MouseMoved,
    point,
    MouseButton::None,
    0,
  )
  .await
  .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  dispatch_mouse(
    page,
    DispatchMouseEventType::MousePressed,
    point,
    MouseButton::Left,
    1,
  )
  .await
  .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  dispatch_mouse(
    page,
    DispatchMouseEventType::MouseReleased,
    point,
    MouseButton::Left,
    0,
  )
  .await
  .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  Ok(())
}

fn upload_failure() -> ErrorData {
  ErrorData::internal_error(
    "Failed to upload file. The element could not accept the file directly, and clicking it did not trigger a file chooser.",
    None,
  )
}
