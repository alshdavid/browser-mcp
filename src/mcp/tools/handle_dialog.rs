use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::DialogType;
use chromiumoxide::cdp::browser_protocol::page::EventJavascriptDialogOpening;
use chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams;
use futures_util::StreamExt as _;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::common::DialogAction;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandleDialogRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Whether to dismiss or accept the dialog")]
  pub action: DialogAction,
  #[schemars(description = "Optional prompt text to enter into the dialog.")]
  pub prompt_text: Option<String>,
}

// How to answer a dialog that opens while another tool call is in flight.
pub(crate) enum DialogResponse {
  Accept,
  Dismiss,
  Prompt(String),
}

impl DialogResponse {
  // A value that is neither verb is the text to answer a window.prompt with.
  pub(crate) fn parse(action: &str) -> Self {
    match action {
      "accept" => DialogResponse::Accept,
      "dismiss" => DialogResponse::Dismiss,
      text => DialogResponse::Prompt(text.to_owned()),
    }
  }

  fn params(&self) -> HandleJavaScriptDialogParams {
    match self {
      DialogResponse::Accept => HandleJavaScriptDialogParams::new(true),
      DialogResponse::Dismiss => HandleJavaScriptDialogParams::new(false),
      DialogResponse::Prompt(text) => {
        let mut params = HandleJavaScriptDialogParams::new(true);
        params.prompt_text = Some(text.clone());
        params
      }
    }
  }
}

// Answers dialogs for as long as a call is in flight. A page that blocks on a
// dialog otherwise stalls that call until the dialog is answered, which nothing
// else can do while the call is still running.
pub(crate) struct DialogWatcher {
  handled: Arc<AtomicBool>,
  stop: Option<oneshot::Sender<()>>,
  task: Option<JoinHandle<()>>,
}

impl DialogWatcher {
  pub(crate) fn start(
    page: Page,
    response: DialogResponse,
    before_unload_only: bool,
  ) -> Self {
    let handled = Arc::new(AtomicBool::new(false));
    let (stop, stop_rx) = oneshot::channel();

    let task = tokio::spawn(watch(
      page,
      response,
      before_unload_only,
      handled.clone(),
      stop_rx,
    ));

    Self {
      handled,
      stop: Some(stop),
      task: Some(task),
    }
  }

  // Stops watching and reports whether a dialog was answered.
  pub(crate) async fn finish(mut self) -> bool {
    if let Some(stop) = self.stop.take() {
      let _ = stop.send(());
    }
    if let Some(task) = self.task.take() {
      let _ = task.await;
    }

    self.handled.load(Ordering::SeqCst)
  }
}

impl Drop for DialogWatcher {
  fn drop(&mut self) {
    if let Some(stop) = self.stop.take() {
      let _ = stop.send(());
    }
  }
}

async fn watch(
  page: Page,
  response: DialogResponse,
  before_unload_only: bool,
  handled: Arc<AtomicBool>,
  mut stop: oneshot::Receiver<()>,
) {
  let Ok(mut events) = page.event_listener::<EventJavascriptDialogOpening>().await else {
    return;
  };

  loop {
    tokio::select! {
      _ = &mut stop => return,
      event = events.next() => {
        let Some(event) = event else {
          return;
        };

        if before_unload_only && event.r#type != DialogType::Beforeunload {
          continue;
        }

        if page.execute(response.params()).await.is_ok() {
          handled.store(true, Ordering::SeqCst);
        }
      }
    }
  }
}

pub async fn handle_dialog(
  tool_context: ToolContext,
  Parameters(HandleDialogRequest {
    page_id,
    action,
    prompt_text,
  }): Parameters<HandleDialogRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context.require_page(page_id).await?;

  if !tool_context.dialog_open(page_id).await {
    return Err(ErrorData::invalid_params("No open dialog found", None));
  }

  let (accept, message) = match action {
    DialogAction::Accept => (true, "Successfully accepted the dialog"),
    DialogAction::Dismiss => (false, "Successfully dismissed the dialog"),
  };

  let mut params = HandleJavaScriptDialogParams::new(accept);
  params.prompt_text = prompt_text;
  // The dialog may already have been dismissed outside of MCP, so a failed CDP
  // call still means it is no longer blocking; erroring would be a false alarm.
  let _ = page.execute(params).await;

  tool_context.clear_dialog(page_id).await;

  Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
}
