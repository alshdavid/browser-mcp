use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use chromiumoxide::cdp::browser_protocol::page::GetNavigationHistoryParams;
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::cdp::browser_protocol::page::NavigateToHistoryEntryParams;
use chromiumoxide::cdp::browser_protocol::page::ReloadParams;
use chromiumoxide::cdp::browser_protocol::page::RemoveScriptToEvaluateOnNewDocumentParams;
use chromiumoxide::error::CdpError;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::common::DialogAction;
use crate::mcp::tools::common::NavigationType;
use crate::mcp::tools::handle_dialog::DialogResponse;
use crate::mcp::tools::handle_dialog::DialogWatcher;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const SETTLE_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NavigatePageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Navigate the page by URL, back or forward in history, or reload.")]
  pub r#type: Option<NavigationType>,
  #[schemars(description = "Target URL (only type=url)")]
  pub url: Option<String>,
  #[schemars(description = "Whether to ignore cache on reload.")]
  pub ignore_cache: Option<bool>,
  #[schemars(
    description = "A JavaScript script to be executed on each new document before any other scripts for the next navigation."
  )]
  pub init_script: Option<String>,
  #[schemars(
    description = "Whether to auto accept or beforeunload dialogs triggered by this navigation. Default is accept."
  )]
  pub handle_before_unload: Option<DialogAction>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn navigate_page(
  tool_context: ToolContext,
  Parameters(NavigatePageRequest {
    page_id,
    r#type,
    url,
    ignore_cache,
    init_script,
    handle_before_unload,
    timeout,
  }): Parameters<NavigatePageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context.require_page(page_id).await?;

  let Some(r#type) = r#type.or_else(|| url.as_ref().map(|_| NavigationType::Url)) else {
    return Err(ErrorData::invalid_params(
      "Either URL or a type is required.",
      None,
    ));
  };

  if matches!(r#type, NavigationType::Url) && url.is_none() {
    return Err(ErrorData::invalid_params(
      "A URL is required for navigation of type=url.",
      None,
    ));
  }

  let timeout = timeout
    .filter(|millis| *millis > 0)
    .map_or(DEFAULT_TIMEOUT, |millis| {
      Duration::from_millis(millis as u64)
    });

  let init_script = match &init_script {
    Some(source) => Some(
      page
        .evaluate_on_new_document(AddScriptToEvaluateOnNewDocumentParams::new(source.clone()))
        .await
        .map_err(internal)?,
    ),
    None => None,
  };

  // Only a beforeunload dialog is auto-answered here; the navigation options
  // leave every other dialog kind to handle_dialog.
  let before_unload = handle_before_unload.unwrap_or(DialogAction::Accept);
  let watcher = DialogWatcher::start(
    page.clone(),
    match before_unload {
      DialogAction::Accept => DialogResponse::Accept,
      DialogAction::Dismiss => DialogResponse::Dismiss,
    },
    true,
  );

  let outcome = run(&page, &r#type, url.as_deref(), ignore_cache, timeout).await;

  let handled = watcher.finish().await;

  if let Some(identifier) = init_script {
    let _ = page
      .execute(RemoveScriptToEvaluateOnNewDocumentParams::new(identifier))
      .await;
  }

  let mut lines = Vec::new();

  match outcome {
    Ok(Action::Url) => lines.push(format!(
      "Successfully navigated to {}.",
      url.unwrap_or_default()
    )),
    Ok(Action::History { url }) => lines.push(format!(
      "Successfully navigated {} to {url}.",
      match r#type {
        NavigationType::Forward => "forward",
        _ => "back",
      }
    )),
    Ok(Action::Reload) => lines.push("Successfully reloaded the page.".to_string()),
    Ok(Action::TimedOut) => lines.push(format!(
      "Timed out after {}ms waiting for the navigation to complete.",
      timeout.as_millis()
    )),
    Err(error) => lines.push(format!("Unable to {}: {error}.", describe(r#type))),
  }

  if handled {
    lines.push(format!(
      "{} a beforeunload dialog.",
      match before_unload {
        DialogAction::Accept => "Accepted",
        DialogAction::Dismiss => "Dismissed",
      }
    ));
    // The watcher that tracks dialogs for handle_dialog also saw this one, but
    // it is already resolved, so it must not read as still open.
    tool_context.clear_dialog(page_id).await;
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(
    lines.join("\n"),
  )]))
}

enum Action {
  Url,
  History { url: String },
  Reload,
  TimedOut,
}

async fn run(
  page: &Page,
  r#type: &NavigationType,
  url: Option<&str>,
  ignore_cache: Option<bool>,
  timeout: Duration,
) -> Result<Action, CdpError> {
  let action = async {
    let action = match r#type {
      NavigationType::Url => {
        page
          .goto(NavigateParams::new(url.unwrap_or_default()))
          .await?;
        Action::Url
      }
      NavigationType::Back => Action::History {
        url: history(page, -1).await?,
      },
      NavigationType::Forward => Action::History {
        url: history(page, 1).await?,
      },
      NavigationType::Reload => {
        reload(page, ignore_cache).await?;
        Action::Reload
      }
    };

    Ok::<Action, CdpError>(action)
  };
  tokio::pin!(action);

  let outcome = tokio::select! {
    result = &mut action => result?,
    _ = tokio::time::sleep(timeout) => Action::TimedOut,
  };

  if !matches!(outcome, Action::TimedOut) {
    settle(page).await;
  }

  Ok(outcome)
}

async fn history(
  page: &Page,
  offset: i64,
) -> Result<String, CdpError> {
  let history = page.execute(GetNavigationHistoryParams::default()).await?;
  let target = history.result.current_index + offset;

  if target < 0 || target as usize >= history.result.entries.len() {
    return Err(CdpError::ChromeMessage(if offset < 0 {
      "no previous entry in history".to_string()
    } else {
      "no next entry in history".to_string()
    }));
  }

  let entry_id = history.result.entries[target as usize].id;
  let url = history.result.entries[target as usize].url.clone();
  page
    .execute(NavigateToHistoryEntryParams::new(entry_id))
    .await?;

  Ok(url)
}

async fn reload(
  page: &Page,
  ignore_cache: Option<bool>,
) -> Result<(), CdpError> {
  let params = ReloadParams {
    ignore_cache,
    ..ReloadParams::default()
  };
  page.execute(params).await?;

  Ok(())
}

async fn settle(page: &Page) {
  let _ = tokio::time::timeout(SETTLE_TIMEOUT, page.wait_for_navigation()).await;
}

fn describe(r#type: NavigationType) -> &'static str {
  match r#type {
    NavigationType::Url => "navigate in the selected page",
    NavigationType::Back => "navigate back in the selected page",
    NavigationType::Forward => "navigate forward in the selected page",
    NavigationType::Reload => "reload the selected page",
  }
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
