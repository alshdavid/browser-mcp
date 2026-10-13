use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum LighthouseMode {
  #[serde(rename = "navigation")]
  Navigation,
  #[serde(rename = "snapshot")]
  Snapshot,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum LighthouseDevice {
  #[serde(rename = "desktop")]
  Desktop,
  #[serde(rename = "mobile")]
  Mobile,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LighthouseAuditRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    default = "mode_default",
    description = "\"navigation\" reloads & audits. \"snapshot\" analyzes current state."
  )]
  pub mode: Option<LighthouseMode>,
  #[schemars(default = "device_default", description = "Device to emulate.")]
  pub device: Option<LighthouseDevice>,
  #[schemars(description = "Directory for reports. If omitted, uses temporary files.")]
  pub output_dir_path: Option<String>,
}

fn mode_default() -> Option<LighthouseMode> {
  Some(LighthouseMode::Navigation)
}

fn device_default() -> Option<LighthouseDevice> {
  Some(LighthouseDevice::Desktop)
}

pub async fn lighthouse_audit(
  tool_context: ToolContext,
  Parameters(LighthouseAuditRequest {
    page_id,
    mode,
    device,
    output_dir_path,
  }): Parameters<LighthouseAuditRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context;
  let _ = (page_id, mode, device, output_dir_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
