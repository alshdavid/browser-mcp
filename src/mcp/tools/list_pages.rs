use chromiumoxide::Page;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

const TITLE_MAX_LENGTH: usize = 50;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPagesRequest {}

pub async fn list_pages(
  tool_context: ToolContext,
  Parameters(ListPagesRequest {}): Parameters<ListPagesRequest>,
) -> Result<CallToolResult, ErrorData> {
  Ok(CallToolResult::success(vec![ContentBlock::text(
    page_list(&tool_context).await?,
  )]))
}

// Shared with select_page, which reports the same list so the caller sees which
// page the call just selected.
pub(crate) async fn page_list(tool_context: &ToolContext) -> Result<String, ErrorData> {
  let pages = tool_context
    .browser_pages()
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  if pages.is_empty() {
    return Ok("No pages found.".to_owned());
  }

  let selected = tool_context.selected_page().await;
  let mut lines = vec!["## Pages".to_owned()];

  for (id, page) in pages {
    let marker = if Some(id) == selected {
      " [selected]"
    } else {
      ""
    };

    lines.push(format!("{id}: {}{marker}", label(&page).await));
  }

  Ok(lines.join("\n"))
}

async fn label(page: &Page) -> String {
  let url = page.url().await.ok().flatten().unwrap_or_default();

  match title(page).await {
    Some(title) if !title.trim().is_empty() => format!("{} ({url})", truncate(title.trim())),
    _ => url,
  }
}

// A page whose session is not ready yet, or that has no document, simply has no
// title to show; the URL still identifies it.
async fn title(page: &Page) -> Option<String> {
  page
    .evaluate_expression("document.title")
    .await
    .ok()
    .and_then(|result| result.into_value::<String>().ok())
}

fn truncate(title: &str) -> String {
  if title.chars().count() <= TITLE_MAX_LENGTH {
    return title.to_owned();
  }

  let kept = title.chars().take(TITLE_MAX_LENGTH - 3).collect::<String>();

  format!("{kept}...")
}
