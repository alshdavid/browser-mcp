use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ScreenshotFormat {
  #[serde(rename = "png")]
  Png,
  #[serde(rename = "jpeg")]
  Jpeg,
  #[serde(rename = "webp")]
  Webp,
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
  Parameters(TakeScreenshotRequest {
    page_id,
    format,
    quality,
    uid,
    full_page,
    file_path,
  }): Parameters<TakeScreenshotRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, format, quality, uid, full_page, file_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
