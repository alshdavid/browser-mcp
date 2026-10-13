use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

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

pub async fn list_network_requests(
  Parameters(ListNetworkRequestsRequest {
    page_id,
    page_size,
    page_idx,
    resource_types,
    include_preserved_requests,
  }): Parameters<ListNetworkRequestsRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (
    page_id,
    page_size,
    page_idx,
    resource_types,
    include_preserved_requests,
  );
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
