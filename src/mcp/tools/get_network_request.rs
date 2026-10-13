use std::path::Path;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::network::GetRequestPostDataParams;
use chromiumoxide::cdp::browser_protocol::network::GetResponseBodyParams;
use chromiumoxide::cdp::browser_protocol::network::RequestId;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::NetworkRequest;
use crate::mcp::tool_context::RedirectHop;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetNetworkRequestRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The reqid of the network request. If omitted returns the currently selected request in the DevTools Network panel."
  )]
  pub reqid: Option<f64>,
  #[schemars(
    description = "The absolute or relative path to a .network-request file to save the request body to. If omitted, the body is returned inline."
  )]
  pub request_file_path: Option<String>,
  #[schemars(
    description = "The absolute or relative path to a .network-response file to save the response body to. If omitted, the body is returned inline."
  )]
  pub response_file_path: Option<String>,
}

const BODY_CONTEXT_SIZE_LIMIT: usize = 10_000;
const URL_CONTEXT_SIZE_LIMIT: usize = 255;

const REQUEST_BODY_NOT_AVAILABLE: &str = "<Request body not available anymore>";
const RESPONSE_BODY_NOT_AVAILABLE: &str = "<Response body not available anymore>";
const EMPTY_RESPONSE: &str = "<empty response>";
const BINARY_DATA: &str = "<binary data>";
const NOT_AVAILABLE_ANYMORE: &str = "<not available anymore>";

pub async fn get_network_request(
  tool_context: ToolContext,
  Parameters(GetNetworkRequestRequest {
    page_id,
    reqid,
    request_file_path,
    response_file_path,
  }): Parameters<GetNetworkRequestRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  // Upstream falls back to the request selected in the DevTools Network panel,
  // which requires the bundled DevTools frontend and has no equivalent here.
  let reqid = reqid.ok_or_else(|| {
    ErrorData::invalid_params(
      "No reqid provided. An explicit reqid from list_network_requests is required: selecting a request in the DevTools Network panel is not supported."
        .to_owned(),
      None,
    )
  })?;

  let request = tool_context
    .network_request(page_id, reqid as u64)
    .await
    .ok_or_else(|| {
      ErrorData::invalid_params(
        format!("No network request found with reqid {reqid} on page {page_id}."),
        None,
      )
    })?;

  let request_body = request_body(&page, &request, request_file_path.as_deref()).await?;
  let response_body = response_body(&page, &request, response_file_path.as_deref()).await;

  Ok(CallToolResult::success(vec![ContentBlock::text(detailed(
    &request,
    request_body,
    response_body,
  ))]))
}

enum Body {
  Inline(String),
  Saved(String),
}

async fn request_body(
  page: &Page,
  request: &NetworkRequest,
  file_path: Option<&str>,
) -> Result<Option<Body>, ErrorData> {
  if !request.has_post_data {
    return Ok(None);
  }

  let post_data = page
    .execute(GetRequestPostDataParams::new(RequestId::new(
      request.cdp_request_id.clone(),
    )))
    .await
    .ok()
    .map(|response| response.result.post_data);

  Ok(Some(match (post_data, file_path) {
    (Some(data), Some(file_path)) => save(file_path, data.as_bytes())?,
    (None, Some(_)) => Body::Inline(REQUEST_BODY_NOT_AVAILABLE.to_owned()),
    (Some(data), None) => Body::Inline(limit(&data, BODY_CONTEXT_SIZE_LIMIT)),
    (None, None) => Body::Inline(REQUEST_BODY_NOT_AVAILABLE.to_owned()),
  }))
}

async fn response_body(
  page: &Page,
  request: &NetworkRequest,
  file_path: Option<&str>,
) -> Option<Body> {
  // Upstream only formats a response body once a response exists; a request
  // that only produced a loadingFailed never gets one.
  request.status?;

  let body = page
    .execute(GetResponseBodyParams::new(RequestId::new(
      request.cdp_request_id.clone(),
    )))
    .await
    .ok()
    .map(|response| response.result);

  match (body, file_path) {
    (Some(body), Some(file_path)) => decode(&body.body, body.base64_encoded)
      .and_then(|bytes| save(file_path, &bytes).ok())
      .or_else(|| Some(Body::Inline(RESPONSE_BODY_NOT_AVAILABLE.to_owned()))),
    (None, Some(_)) => Some(Body::Inline(RESPONSE_BODY_NOT_AVAILABLE.to_owned())),
    (None, None) => Some(Body::Inline(NOT_AVAILABLE_ANYMORE.to_owned())),
    (Some(body), None) => Some(match decode(&body.body, body.base64_encoded) {
      Some(bytes) => match String::from_utf8(bytes) {
        Ok(text) if text.is_empty() => Body::Inline(EMPTY_RESPONSE.to_owned()),
        Ok(text) => Body::Inline(limit(&text, BODY_CONTEXT_SIZE_LIMIT)),
        Err(_) => Body::Inline(BINARY_DATA.to_owned()),
      },
      None => Body::Inline(BINARY_DATA.to_owned()),
    }),
  }
}

fn save(
  file_path: &str,
  contents: &[u8],
) -> Result<Body, ErrorData> {
  let path = Path::new(file_path);

  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent)
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  std::fs::write(path, contents)
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  Ok(Body::Saved(path.to_string_lossy().into_owned()))
}

fn detailed(
  request: &NetworkRequest,
  request_body: Option<Body>,
  response_body: Option<Body>,
) -> String {
  let mut lines = vec![
    format!("## Request {}", request.url),
    format!("Status: {}", status(request)),
    "### Request Headers".to_owned(),
  ];

  lines.extend(headers(&request.request_headers));

  if let Some(body) = body_section("### Request Body", request_body) {
    lines.extend(body);
  }

  if !request.response_headers.is_empty() {
    lines.push("### Response Headers".to_owned());
    lines.extend(headers(&request.response_headers));
  }

  if let Some(body) = body_section("### Response Body", response_body) {
    lines.extend(body);
  }

  if let Some(error_text) = request.error_text.as_ref() {
    lines.push("### Request failed with".to_owned());
    lines.push(error_text.clone());
  }

  if !request.redirect_chain.is_empty() {
    lines.push("### Redirect chain".to_owned());
    lines.extend(
      request
        .redirect_chain
        .iter()
        .enumerate()
        .map(|(depth, hop)| format!("{}{}", "  ".repeat(depth), concise(hop))),
    );
  }

  lines.join("\n")
}

fn body_section(
  heading: &str,
  body: Option<Body>,
) -> Option<Vec<String>> {
  let body = body?;

  Some(vec![
    heading.to_owned(),
    match body {
      Body::Inline(text) => text,
      Body::Saved(path) => format!("Saved to {path}."),
    },
  ])
}

fn status(request: &NetworkRequest) -> String {
  if let Some(status) = request.status {
    return status.to_string();
  }

  match request.error_text.as_ref() {
    Some(error_text) => error_text.clone(),
    None => "pending".to_owned(),
  }
}

fn headers(headers: &[(String, String)]) -> Vec<String> {
  headers
    .iter()
    .map(|(name, value)| format!("- {name}:{value}"))
    .collect()
}

fn concise(hop: &RedirectHop) -> String {
  format!(
    "reqid={} {} {} [{}]",
    hop.reqid,
    hop.method,
    limit(&hop.url, URL_CONTEXT_SIZE_LIMIT),
    hop.status
  )
}

fn limit(
  text: &str,
  size_limit: usize,
) -> String {
  if text.chars().count() > size_limit {
    let truncated: String = text.chars().take(size_limit).collect();
    return format!("{truncated}... <truncated>");
  }

  text.to_owned()
}

fn decode(
  body: &str,
  base64_encoded: bool,
) -> Option<Vec<u8>> {
  if base64_encoded {
    return base64_decode(body);
  }

  Some(body.as_bytes().to_vec())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
  let mut output = Vec::with_capacity(input.len() / 4 * 3);
  let mut accumulator = 0u32;
  let mut bits = 0u32;

  for byte in input.bytes() {
    let value = match byte {
      b'A'..=b'Z' => byte - b'A',
      b'a'..=b'z' => byte - b'a' + 26,
      b'0'..=b'9' => byte - b'0' + 52,
      b'+' => 62,
      b'/' => 63,
      b'=' => break,
      b'\n' | b'\r' => continue,
      _ => return None,
    };

    accumulator = (accumulator << 6) | u32::from(value);
    bits += 6;

    if bits >= 8 {
      bits -= 8;
      output.push((accumulator >> bits) as u8);
    }
  }

  Some(output)
}
