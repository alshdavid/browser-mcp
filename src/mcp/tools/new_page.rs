use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::target::CreateBrowserContextParams;
use chromiumoxide::cdp::browser_protocol::target::CreateTargetParams;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPageRequest {
  #[schemars(description = "URL to load in a new page.")]
  pub url: String,
  #[schemars(
    description = "Whether to open the page in the background without bringing it to the front. Default is false (foreground)."
  )]
  pub background: Option<bool>,
  #[schemars(
    description = "If specified, the page is created in an isolated browser context with the given name. Pages in the same browser context share cookies and storage. Pages in different browser contexts are fully isolated (useful for clean-slate testing of cookies and authentication)."
  )]
  pub isolated_context: Option<String>,
  #[schemars(
    description = "Maximum wait time in milliseconds. If set to 0, the default timeout will be used."
  )]
  pub timeout: Option<i64>,
}

pub async fn new_page(
  tool_context: ToolContext,
  Parameters(NewPageRequest {
    url,
    background,
    isolated_context,
    timeout,
  }): Parameters<NewPageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let timeout = timeout
    .filter(|millis| *millis > 0)
    .map(|millis| Duration::from_millis(millis as u64));

  let browser_context_id = match isolated_context {
    Some(name) => Some(match tool_context.isolated_context(&name).await {
      Some(id) => id,
      None => {
        let id = tool_context
          .with_browser(async |browser| {
            browser
              .create_browser_context(CreateBrowserContextParams::default())
              .await
          })
          .await
          .map_err(internal)?
          .map_err(internal)?;
        tool_context.set_isolated_context(name, id.clone()).await;
        id
      }
    }),
    None => None,
  };

  let page = tool_context
    .with_browser(async |browser| {
      let params = CreateTargetParams {
        browser_context_id,
        background,
        ..CreateTargetParams::new(&url)
      };
      browser.new_page(params).await
    })
    .await
    .map_err(internal)?
    .map_err(internal)?;

  settle(&page, &url, background, timeout).await;

  let id = tool_context.register_page(page.clone()).await;
  let url = page.url().await.ok().flatten().unwrap_or_default();

  Ok(CallToolResult::success(vec![ContentBlock::text(
    serde_json::json!({ "id": id, "url": url }).to_string(),
  )]))
}

async fn settle(
  page: &Page,
  url: &str,
  background: Option<bool>,
  timeout: Option<Duration>,
) {
  if url.is_empty() {
    return;
  }

  let navigation = page.wait_for_navigation();
  let navigation = match timeout {
    Some(timeout) => match tokio::time::timeout(timeout, navigation).await {
      Ok(result) => result,
      Err(_) => return,
    },
    None => navigation.await,
  };

  if navigation.is_err() && !background.unwrap_or(false) {
    let _ = page.goto("about:blank").await;
  }
}

fn internal(error: chromiumoxide::error::CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
