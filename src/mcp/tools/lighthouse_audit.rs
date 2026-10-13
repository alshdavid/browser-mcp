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

const UNAVAILABLE_MESSAGE: &str = "\
The lighthouse_audit tool is not available in this build.

Lighthouse is not a Chrome DevTools Protocol feature: upstream implements this tool by running the Lighthouse engine, which ships as a 3.5 MB JavaScript bundle executed through a Node.js runtime. This server has no JavaScript runtime and no Rust port of the Lighthouse engine, so audit scores for accessibility, SEO, best practices, or agentic browsing cannot be produced here. Reporting fabricated or partial scores instead would be misleading, so this tool returns no results at all.

What to do instead:
- For performance work, use performance_start_trace: it is fully implemented, records a real trace over CDP, and reports Core Web Vitals plus Insights that performance_analyze_insight can then break down.
- For accessibility and SEO structure, read the page directly with take_snapshot (or evaluate_script) and inspect the document yourself; the required signals, such as landmarks, headings, alt text, labels, link text, and meta tags, are all readable from the DOM.
- To check best practices, use the network and console tools (list_network_requests, get_network_request, list_console_messages) to surface mixed content, failed requests, deprecated APIs, and browser warnings.
- If you need genuine Lighthouse results, run Lighthouse outside this server against the same URL and feed the report back to the agent.

Repeated calls will return this same message.";

pub async fn lighthouse_audit(
  tool_context: ToolContext,
  Parameters(LighthouseAuditRequest {
    page_id,
    mode: _mode,
    device: _device,
    output_dir_path: _output_dir_path,
  }): Parameters<LighthouseAuditRequest>,
) -> Result<CallToolResult, ErrorData> {
  if tool_context.page(page_id).await.is_none() {
    return Err(ErrorData::invalid_params(
      format!("No page found with id {page_id}"),
      None,
    ));
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(
    UNAVAILABLE_MESSAGE,
  )]))
}
