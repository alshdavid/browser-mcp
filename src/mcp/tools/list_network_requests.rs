use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::NetworkRequest;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ResourceType {
  #[serde(rename = "document")]
  Document,
  #[serde(rename = "stylesheet")]
  Stylesheet,
  #[serde(rename = "image")]
  Image,
  #[serde(rename = "media")]
  Media,
  #[serde(rename = "font")]
  Font,
  #[serde(rename = "script")]
  Script,
  #[serde(rename = "texttrack")]
  Texttrack,
  #[serde(rename = "xhr")]
  Xhr,
  #[serde(rename = "fetch")]
  Fetch,
  #[serde(rename = "prefetch")]
  Prefetch,
  #[serde(rename = "eventsource")]
  Eventsource,
  #[serde(rename = "websocket")]
  Websocket,
  #[serde(rename = "manifest")]
  Manifest,
  #[serde(rename = "signedexchange")]
  Signedexchange,
  #[serde(rename = "ping")]
  Ping,
  #[serde(rename = "cspviolationreport")]
  Cspviolationreport,
  #[serde(rename = "preflight")]
  Preflight,
  #[serde(rename = "fedcm")]
  Fedcm,
  #[serde(rename = "other")]
  Other,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListNetworkRequestsRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Maximum number of requests to return. When omitted, returns all requests."
  )]
  pub page_size: Option<i64>,
  #[schemars(
    description = "Page number to return (0-based). When omitted, returns the first page."
  )]
  pub page_idx: Option<i64>,
  #[schemars(
    description = "Filter requests to only return requests of the specified resource types. When omitted or empty, returns all requests."
  )]
  pub resource_types: Option<Vec<ResourceType>>,
  #[schemars(
    description = "Set to true to return the preserved requests over the last 3 navigations."
  )]
  pub include_preserved_requests: Option<bool>,
}

const DEFAULT_PAGE_SIZE: usize = 20;
const URL_SIZE_LIMIT: usize = 255;

impl ResourceType {
  fn as_str(self) -> &'static str {
    match self {
      ResourceType::Document => "Document",
      ResourceType::Stylesheet => "Stylesheet",
      ResourceType::Image => "Image",
      ResourceType::Media => "Media",
      ResourceType::Font => "Font",
      ResourceType::Script => "Script",
      ResourceType::Texttrack => "TextTrack",
      ResourceType::Xhr => "XHR",
      ResourceType::Fetch => "Fetch",
      ResourceType::Prefetch => "Prefetch",
      ResourceType::Eventsource => "EventSource",
      ResourceType::Websocket => "WebSocket",
      ResourceType::Manifest => "Manifest",
      ResourceType::Signedexchange => "SignedExchange",
      ResourceType::Ping => "Ping",
      ResourceType::Cspviolationreport => "CSPViolationReport",
      ResourceType::Preflight => "Preflight",
      ResourceType::Fedcm => "FedCM",
      ResourceType::Other => "Other",
    }
  }
}

pub async fn list_network_requests(
  tool_context: ToolContext,
  Parameters(ListNetworkRequestsRequest {
    page_id,
    page_size,
    page_idx,
    resource_types,
    include_preserved_requests,
  }): Parameters<ListNetworkRequestsRequest>,
) -> Result<CallToolResult, ErrorData> {
  tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  let mut requests = tool_context
    .network_requests(page_id, include_preserved_requests.unwrap_or(false))
    .await;

  if let Some(resource_types) = resource_types
    .as_ref()
    .filter(|resource_types| !resource_types.is_empty())
  {
    requests.retain(|request| {
      resource_types
        .iter()
        .any(|kind| request.resource_type.as_deref() == Some(kind.as_str()))
    });
  }

  let pagination = paginate(&requests, page_size, page_idx);

  let mut lines = vec!["## Network requests".to_owned()];

  if requests.is_empty() {
    lines.push("No requests found.".to_owned());
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      lines.join("\n"),
    )]));
  }

  lines.extend(pagination.info);
  lines.extend(pagination.items.iter().map(format_request));

  Ok(CallToolResult::success(vec![ContentBlock::text(
    lines.join("\n"),
  )]))
}

struct Pagination {
  items: Vec<NetworkRequest>,
  info: Vec<String>,
}

fn paginate(
  requests: &[NetworkRequest],
  page_size: Option<i64>,
  page_idx: Option<i64>,
) -> Pagination {
  let Some(page_size) = page_size.or(page_idx.map(|_| DEFAULT_PAGE_SIZE as i64)) else {
    return Pagination {
      items: requests.to_vec(),
      info: vec![showing(0, requests.len(), 0, 1, requests.len())],
    };
  };

  let page_size = page_size.max(1) as usize;
  let total_pages = requests.len().div_ceil(page_size).max(1);
  let requested = page_idx.unwrap_or(0);
  let invalid_page = requested < 0 || requested as usize >= total_pages;
  let current_page = if invalid_page { 0 } else { requested as usize };

  let start = current_page * page_size;
  let end = (start + page_size).min(requests.len());
  let items = requests[start.min(requests.len())..end].to_vec();

  let mut info = Vec::new();
  if invalid_page {
    info.push("Invalid page number provided. Showing first page.".to_owned());
  }

  info.push(showing(
    start,
    end,
    current_page,
    total_pages,
    requests.len(),
  ));

  if current_page < total_pages - 1 {
    info.push(format!("Next page: {}", current_page + 1));
  }

  if current_page > 0 {
    info.push(format!("Previous page: {}", current_page - 1));
  }

  Pagination { items, info }
}

fn showing(
  start: usize,
  end: usize,
  current_page: usize,
  total_pages: usize,
  total: usize,
) -> String {
  format!(
    "Showing {}-{} of {} (Page {} of {}).",
    if total == 0 { 0 } else { start + 1 },
    end,
    total,
    current_page + 1,
    total_pages
  )
}

fn format_request(request: &NetworkRequest) -> String {
  let status = if let Some(status) = request.status {
    status.to_string()
  } else if let Some(error_text) = request.error_text.as_ref() {
    error_text.clone()
  } else {
    "pending".to_owned()
  };

  // Long URLs (e.g. data: URLs) bloat the concise list output; the detailed
  // view keeps the untruncated URL.
  let url = truncate(&request.url, URL_SIZE_LIMIT);

  format!(
    "reqid={} {} {} [{}]",
    request.id, request.method, url, status
  )
}

fn truncate(
  text: &str,
  size_limit: usize,
) -> String {
  if text.chars().count() > size_limit {
    let truncated: String = text.chars().take(size_limit).collect();
    return format!("{truncated}... <truncated>");
  }
  text.to_owned()
}
