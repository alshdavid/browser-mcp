use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::js_protocol::heap_profiler::EventAddHeapSnapshotChunk;
use chromiumoxide::cdp::js_protocol::heap_profiler::TakeHeapSnapshotParams;
use chromiumoxide::error::CdpError;
use futures_util::StreamExt as _;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

const HEAP_SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakeHeapsnapshotRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "A path to a .heapsnapshot file to save the heapsnapshot to.")]
  pub file_path: String,
}

pub async fn take_heapsnapshot(
  tool_context: ToolContext,
  Parameters(TakeHeapsnapshotRequest { page_id, file_path }): Parameters<TakeHeapsnapshotRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context.require_page(page_id).await?;

  let path = with_heapsnapshot_extension(&file_path);
  let snapshot = capture(&page).await?;

  save(&path, snapshot.as_bytes()).await?;

  Ok(CallToolResult::success(vec![ContentBlock::text(format!(
    "Heap snapshot saved to {}",
    path.to_string_lossy()
  ))]))
}

async fn capture(page: &Page) -> Result<String, ErrorData> {
  // takeHeapSnapshot reports completion only by resolving; the chunks arrive as
  // events while it is in flight, so the listener has to exist before the call.
  let mut chunks = page
    .event_listener::<EventAddHeapSnapshotChunk>()
    .await
    .map_err(internal)?;

  let mut snapshot = Vec::new();

  let collection = async {
    loop {
      {
        // Draining is bounded by the command itself rather than by the stream
        // ending, because the stream never ends and a chunk can still be in
        // flight after the command resolves. The select is biased so that a
        // ready command wins over a ready chunk, keeping the completion check
        // ahead of the drain on every iteration.
        let next = tokio::select! {
          biased;
          result = page.execute(TakeHeapSnapshotParams::default()) => return result.map(|_| ()).map_err(internal),
          chunk = chunks.next() => chunk.map(|event| event.chunk.clone()),
        };

        match next {
          Some(chunk) => snapshot.extend_from_slice(chunk.as_bytes()),
          None => return Ok(()),
        }
      }
    }
  };

  match tokio::time::timeout(HEAP_SNAPSHOT_TIMEOUT, collection).await {
    Ok(outcome) => outcome?,
    Err(_) => {
      return Err(ErrorData::internal_error(
        "Timed out capturing the heap snapshot.".to_owned(),
        None,
      ));
    }
  }

  String::from_utf8(snapshot).map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn with_heapsnapshot_extension(file_path: &str) -> PathBuf {
  let path = Path::new(file_path);
  let mut filename = path.file_name().map(PathBuf::from).unwrap_or_default();
  filename.set_extension("heapsnapshot");

  match path.parent() {
    Some(parent) => parent.join(filename),
    None => filename,
  }
}

async fn save(
  path: &Path,
  contents: &[u8],
) -> Result<(), ErrorData> {
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  tokio::fs::write(path, contents)
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
