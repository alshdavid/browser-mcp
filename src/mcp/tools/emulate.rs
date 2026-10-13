use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum NetworkConditions {
  #[serde(rename = "Offline")]
  Offline,
  #[serde(rename = "Slow 3G")]
  Slow3g,
  #[serde(rename = "Fast 3G")]
  Fast3g,
  #[serde(rename = "Slow 4G")]
  Slow4g,
  #[serde(rename = "Fast 4G")]
  Fast4g,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ColorScheme {
  #[serde(rename = "dark")]
  Dark,
  #[serde(rename = "light")]
  Light,
  #[serde(rename = "auto")]
  Auto,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmulateRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Throttle network. Omit to disable throttling.")]
  pub network_conditions: Option<NetworkConditions>,
  #[schemars(
    description = "Represents the CPU slowdown factor. Omit or set the rate to 1 to disable throttling"
  )]
  pub cpu_throttling_rate: Option<f64>,
  #[schemars(
    description = "Geolocation (`<latitude>,<longitude>`) to emulate. Latitude between -90 and 90. Longitude between -180 and 180. Omit to clear the geolocation override."
  )]
  pub geolocation: Option<String>,
  #[schemars(
    description = "User agent to emulate. Set to empty string to clear the user agent override."
  )]
  pub user_agent: Option<String>,
  #[schemars(
    description = "Emulate the dark or the light mode. Set to \"auto\" to reset to the default."
  )]
  pub color_scheme: Option<ColorScheme>,
  #[schemars(
    description = "Emulate device viewports '<width>x<height>x<devicePixelRatio>[,mobile][,touch][,landscape]'. 'touch' and 'mobile' to emulate mobile devices. 'landscape' to emulate landscape mode."
  )]
  pub viewport: Option<String>,
  #[schemars(
    description = "Extra HTTP headers as a JSON string object, e.g. {\"X-Custom\": \"value\", \"Authorization\": \"Bearer token\"}. Headers are included into every HTTP request originating from the page and persist across navigations until cleared. Pass an empty string to clear all extra headers."
  )]
  pub extra_http_headers: Option<String>,
}

pub async fn emulate(
  Parameters(EmulateRequest {
    page_id,
    network_conditions,
    cpu_throttling_rate,
    geolocation,
    user_agent,
    color_scheme,
    viewport,
    extra_http_headers,
  }): Parameters<EmulateRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (
    page_id,
    network_conditions,
    cpu_throttling_rate,
    geolocation,
    user_agent,
    color_scheme,
    viewport,
    extra_http_headers,
  );
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
