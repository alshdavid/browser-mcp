use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::browser::Bounds;
use chromiumoxide::cdp::browser_protocol::browser::GetWindowForTargetParams;
use chromiumoxide::cdp::browser_protocol::browser::SetWindowBoundsParams;
use chromiumoxide::cdp::browser_protocol::browser::WindowState;
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::error::CdpError;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResizePageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Page width")]
  pub width: f64,
  #[schemars(description = "Page height")]
  pub height: f64,
}

pub async fn resize_page(
  tool_context: ToolContext,
  Parameters(ResizePageRequest {
    page_id,
    width,
    height,
  }): Parameters<ResizePageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  let width = width.round().max(0.0) as i64;
  let height = height.round().max(0.0) as i64;

  leave_fullscreen(&page).await;

  let params = SetDeviceMetricsOverrideParams::builder()
    .width(width)
    .height(height)
    .device_scale_factor(0.0)
    .mobile(false)
    .build()
    .map_err(|error| ErrorData::internal_error(error, None))?;

  page.execute(params).await.map_err(internal)?;

  Ok(CallToolResult::success(vec![ContentBlock::text(format!(
    "Successfully resized the page to {width}x{height}."
  ))]))
}

// The window bounds API is unsupported on some platforms, and the page still
// gets the requested viewport through the device metrics override.
async fn leave_fullscreen(page: &Page) {
  let Ok(current) = page.execute(GetWindowForTargetParams::default()).await else {
    return;
  };

  if current.result.bounds.window_state == Some(WindowState::Normal) {
    return;
  }

  let bounds = Bounds {
    window_state: Some(WindowState::Normal),
    ..Bounds::default()
  };
  let params = SetWindowBoundsParams {
    window_id: current.result.window_id,
    bounds,
  };

  let _ = page.execute(params.clone()).await;
  // Ubuntu needs the "normal" state set twice to actually leave fullscreen.
  let _ = page.execute(params).await;
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
