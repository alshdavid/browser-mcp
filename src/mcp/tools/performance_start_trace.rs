use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::error::CdpError;
use flate2::Compression;
use flate2::write::GzEncoder;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use super::performance_trace;
use crate::mcp::tool_context::ToolContext;

const AUTO_STOP_DELAY: Duration = Duration::from_secs(5);

// Puppeteer's bufferSize is expressed in bytes; the CDP traceConfig field is in
// kilobytes, so upstream's 1200 * 1000 bytes lands on 1200 KB.
const BUFFER_SIZE_IN_KB: f64 = 1200.0;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceStartTraceRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    default = "reload_default",
    description = "Determines if, once tracing has started, the target page should be automatically reloaded. Navigate the page to the right URL using the navigate_page tool BEFORE starting the trace if reload or autoStop is set to true."
  )]
  pub reload: Option<bool>,
  #[schemars(
    default = "auto_stop_default",
    description = "Determines if the trace recording should be automatically stopped."
  )]
  pub auto_stop: Option<bool>,
  #[schemars(
    description = "The absolute file path, or a file path relative to the current working directory, to save the raw trace data. For example, trace.json.gz (compressed) or trace.json (uncompressed)."
  )]
  pub file_path: Option<String>,
}

fn reload_default() -> Option<bool> {
  Some(true)
}

fn auto_stop_default() -> Option<bool> {
  Some(true)
}

pub async fn performance_start_trace(
  tool_context: ToolContext,
  Parameters(PerformanceStartTraceRequest {
    page_id,
    reload,
    auto_stop,
    file_path,
  }): Parameters<PerformanceStartTraceRequest>,
) -> Result<CallToolResult, ErrorData> {
  if tool_context.is_running_performance_trace().await {
    return Err(ErrorData::invalid_params(
      "Error: a performance trace is already running. Use performance_stop_trace to stop it. Only one trace can be running at any given time.",
      None,
    ));
  }

  let page = tool_context.require_page(page_id).await?;

  tool_context.set_running_performance_trace(true).await;

  let reload = reload.unwrap_or(true);
  let auto_stop = auto_stop.unwrap_or(true);
  let url = page.url().await.ok().flatten().unwrap_or_default();

  match run(
    &tool_context,
    &page,
    reload,
    auto_stop,
    url.as_str(),
    file_path.as_deref(),
  )
  .await
  {
    Ok(lines) => Ok(CallToolResult::success(
      lines.into_iter().map(ContentBlock::text).collect(),
    )),
    Err(error) => {
      // Without this the flag stays set for the rest of the session whenever a
      // setup step fails before the trace is stopped, blocking every later trace.
      if tool_context.is_running_performance_trace().await {
        let _ = performance_trace::stop(&page).await;
        tool_context.set_running_performance_trace(false).await;
      }
      Err(error)
    }
  }
}

pub async fn stop_and_record(
  tool_context: &ToolContext,
  page: &Page,
  file_path: Option<&str>,
) -> Result<Vec<String>, ErrorData> {
  let mut lines = Vec::new();

  let outcome = performance_trace::stop(page).await;
  tool_context.set_running_performance_trace(false).await;
  let raw = outcome?;

  if let Some(file_path) = file_path
    && !raw.is_empty()
  {
    lines.push(format!(
      "The raw trace data was saved to {}.",
      save(&raw, file_path)?
    ));
  }

  let url = page.url().await.ok().flatten().unwrap_or_default();
  let trace = performance_trace::parse_recording(url, raw);
  tool_context.store_trace_recording(trace).await;

  if let Some(trace) = tool_context.last_recorded_trace().await {
    lines.push("The performance trace has been stopped.".to_owned());
    lines.push(performance_trace::describe(&trace));

    let distinct = performance_trace::event_names(&trace).len();
    if distinct > 0 {
      lines.push(format!("{distinct} distinct event names were recorded."));
    }
  }

  Ok(lines)
}

async fn run(
  tool_context: &ToolContext,
  page: &Page,
  reload: bool,
  auto_stop: bool,
  url: &str,
  file_path: Option<&str>,
) -> Result<Vec<String>, ErrorData> {
  if reload {
    // about:blank first so the recorded navigation starts from a clean slate.
    page
      .goto(NavigateParams::new("about:blank"))
      .await
      .map_err(internal)?;
  }

  performance_trace::start(page, BUFFER_SIZE_IN_KB)
    .await
    .map_err(internal)?;

  if reload {
    page
      .goto(NavigateParams::new(url))
      .await
      .map_err(internal)?;
  }

  if !auto_stop {
    return Ok(vec![
      "The performance trace is being recorded. Use performance_stop_trace to stop it.".to_owned(),
    ]);
  }

  tokio::time::sleep(AUTO_STOP_DELAY).await;

  stop_and_record(tool_context, page, file_path).await
}

fn save(
  raw: &[u8],
  file_path: &str,
) -> Result<String, ErrorData> {
  let path = with_json_extension(file_path);
  let data = if path.extension().is_some_and(|extension| extension == "gz") {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
      .write_all(raw)
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
    encoder
      .finish()
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?
  } else {
    raw.to_vec()
  };

  write(&path, &data)?;
  Ok(path.to_string_lossy().into_owned())
}

fn with_json_extension(file_path: &str) -> PathBuf {
  let path = Path::new(file_path);
  let mut filename = path.file_name().map(PathBuf::from).unwrap_or_default();
  if filename.extension().is_none() {
    filename.set_extension("json");
  }

  match path.parent() {
    Some(parent) => parent.join(filename),
    None => filename,
  }
}

fn write(
  path: &Path,
  contents: &[u8],
) -> Result<(), ErrorData> {
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent)
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  std::fs::write(path, contents).map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
