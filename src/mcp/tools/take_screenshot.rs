use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use base64::Engine as _;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::GetBoxModelParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::cdp::browser_protocol::page::Viewport;
use chromiumoxide::layout::ElementQuad;
use chromiumoxide::page::ScreenshotParams;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

// Upstream writes a capture this large to a file rather than inlining megabytes
// of base64 into the response.
const INLINE_LIMIT: usize = 2_000_000;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ScreenshotFormat {
  #[serde(rename = "png")]
  Png,
  #[serde(rename = "jpeg")]
  Jpeg,
  #[serde(rename = "webp")]
  Webp,
}

impl ScreenshotFormat {
  fn capture(self) -> CaptureScreenshotFormat {
    match self {
      ScreenshotFormat::Png => CaptureScreenshotFormat::Png,
      ScreenshotFormat::Jpeg => CaptureScreenshotFormat::Jpeg,
      ScreenshotFormat::Webp => CaptureScreenshotFormat::Webp,
    }
  }

  fn mime(self) -> &'static str {
    match self {
      ScreenshotFormat::Png => "image/png",
      ScreenshotFormat::Jpeg => "image/jpeg",
      ScreenshotFormat::Webp => "image/webp",
    }
  }

  fn extension(self) -> &'static str {
    match self {
      ScreenshotFormat::Png => "png",
      ScreenshotFormat::Jpeg => "jpeg",
      ScreenshotFormat::Webp => "webp",
    }
  }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakeScreenshotRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    default = "png_default",
    description = "Type of format to save the screenshot as. Default is \"png\""
  )]
  pub format: Option<ScreenshotFormat>,
  #[schemars(
    description = "Compression quality for JPEG and WebP formats (0-100). Higher values mean better quality but larger file sizes. Ignored for PNG format."
  )]
  pub quality: Option<f64>,
  #[schemars(
    description = "The uid of an element on the page from the page content snapshot. If omitted, takes a page screenshot."
  )]
  pub uid: Option<String>,
  #[schemars(
    description = "If set to true takes a screenshot of the full page instead of the currently visible viewport. Incompatible with uid."
  )]
  pub full_page: Option<bool>,
  #[schemars(
    description = "The absolute path, or a path relative to the current working directory, to save the screenshot to instead of attaching it to the response."
  )]
  pub file_path: Option<String>,
}

fn png_default() -> Option<ScreenshotFormat> {
  Some(ScreenshotFormat::Png)
}

pub async fn take_screenshot(
  tool_context: ToolContext,
  Parameters(TakeScreenshotRequest {
    page_id,
    format,
    quality,
    uid,
    full_page,
    file_path,
  }): Parameters<TakeScreenshotRequest>,
) -> Result<CallToolResult, ErrorData> {
  let full_page = full_page.unwrap_or(false);

  if uid.is_some() && full_page {
    return Err(ErrorData::invalid_params(
      "Providing both \"uid\" and \"fullPage\" is not allowed.",
      None,
    ));
  }

  let page = tool_context.require_page(page_id).await?;
  let format = format.unwrap_or(ScreenshotFormat::Png);

  let mut params = ScreenshotParams::builder().format(format.capture());

  // Quality only means something for the lossy formats.
  if !matches!(format, ScreenshotFormat::Png)
    && let Some(quality) = quality
  {
    params = params.quality(quality.round().clamp(0.0, 100.0) as i64);
  }

  let (bytes, description) = match uid.as_deref() {
    Some(uid) => {
      let clip = element_clip(&tool_context, page_id, &page, uid).await?;
      let bytes = page
        .screenshot(params.clip(clip).capture_beyond_viewport(true).build())
        .await
        .map_err(internal)?;

      (
        bytes,
        format!("Took a screenshot of node with uid \"{uid}\"."),
      )
    }
    None if full_page => {
      let clip = full_page_clip(&page).await?;
      let bytes = page
        .screenshot(params.clip(clip).capture_beyond_viewport(true).build())
        .await
        .map_err(internal)?;

      (
        bytes,
        "Took a screenshot of the full current page.".to_owned(),
      )
    }
    None => {
      let bytes = page.screenshot(params.build()).await.map_err(internal)?;

      (
        bytes,
        "Took a screenshot of the current page's viewport.".to_owned(),
      )
    }
  };

  let mut lines = vec![description];

  if let Some(file_path) = file_path {
    let path = with_screenshot_extension(&file_path, format.extension());
    save(&path, &bytes).await?;
    lines.push(format!("Saved screenshot to {}.", path.display()));

    return Ok(CallToolResult::success(vec![ContentBlock::text(
      lines.join("\n"),
    )]));
  }

  if bytes.len() >= INLINE_LIMIT {
    let path = temporary_screenshot_path(format.extension());
    save(&path, &bytes).await?;
    lines.push(format!("Saved screenshot to {}.", path.display()));

    return Ok(CallToolResult::success(vec![ContentBlock::text(
      lines.join("\n"),
    )]));
  }

  Ok(CallToolResult::success(vec![
    ContentBlock::text(lines.join("\n")),
    ContentBlock::image(
      base64::engine::general_purpose::STANDARD.encode(&bytes),
      format.mime(),
    ),
  ]))
}

// The browser's full-page screenshot is implemented by overriding the device
// metrics to the whole document, which it never restores. Clipping to the
// content size beyond the viewport reaches the same pixels without leaving the
// page emulated at that size.
async fn full_page_clip(page: &Page) -> Result<Viewport, ErrorData> {
  let size = page
    .layout_metrics()
    .await
    .map_err(internal)?
    .css_content_size;

  Ok(Viewport {
    x: 0.0,
    y: 0.0,
    width: size.width,
    height: size.height,
    scale: 1.0,
  })
}

async fn element_clip(
  tool_context: &ToolContext,
  page_id: f64,
  page: &Page,
  uid: &str,
) -> Result<Viewport, ErrorData> {
  let backend_node_id = tool_context
    .backend_node_for_uid(page_id, uid)
    .await
    .ok_or_else(|| {
      ErrorData::invalid_params(
        format!("Element uid \"{uid}\" not found on page {page_id}."),
        None,
      )
    })?;

  let model = page
    .execute(
      GetBoxModelParams::builder()
        .backend_node_id(BackendNodeId::new(backend_node_id))
        .build(),
    )
    .await
    .map_err(internal)?
    .result
    .model;

  let quad = ElementQuad::from_quad(&model.border);
  let x = quad.most_left();
  let y = quad.most_top();

  if quad.quad_area() <= 1.0 {
    return Err(ErrorData::invalid_params(
      format!("Element uid \"{uid}\" has no visible box to capture."),
      None,
    ));
  }

  // Box model quads are viewport relative while a clip is in page coordinates,
  // so the scroll offset the quad was measured under has to be added back.
  let scroll = page
    .layout_metrics()
    .await
    .map_err(internal)?
    .css_layout_viewport;

  Ok(Viewport {
    x: x + scroll.page_x as f64,
    y: y + scroll.page_y as f64,
    width: quad.most_right() - x,
    height: quad.most_bottom() - y,
    scale: 1.0,
  })
}

fn with_screenshot_extension(
  path: &str,
  extension: &str,
) -> PathBuf {
  Path::new(path).with_extension(extension)
}

fn temporary_screenshot_path(extension: &str) -> PathBuf {
  let stamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|elapsed| elapsed.as_nanos())
    .unwrap_or_default();

  std::env::temp_dir().join(format!("screenshot-{stamp}.{extension}"))
}

async fn save(
  path: &Path,
  bytes: &[u8],
) -> Result<(), ErrorData> {
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  tokio::fs::write(path, bytes)
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn internal(error: chromiumoxide::error::CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
