use std::collections::BTreeMap;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::emulation::ClearDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::emulation::MediaFeature;
use chromiumoxide::cdp::browser_protocol::emulation::ScreenOrientation;
use chromiumoxide::cdp::browser_protocol::emulation::ScreenOrientationType;
use chromiumoxide::cdp::browser_protocol::emulation::SetCpuThrottlingRateParams;
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::emulation::SetEmulatedMediaParams;
use chromiumoxide::cdp::browser_protocol::emulation::SetGeolocationOverrideParams;
use chromiumoxide::cdp::browser_protocol::emulation::SetTouchEmulationEnabledParams;
// Network.emulateNetworkConditions is deprecated in favour of the
// emulateNetworkConditionsByRule + overrideNetworkState pair, but only the
// deprecated method throttles navigator.connection the way the emulate tool
// needs, so it stays until the replacement is verified equivalent.
#[allow(deprecated)]
use chromiumoxide::cdp::browser_protocol::network::EmulateNetworkConditionsParams;
use chromiumoxide::cdp::browser_protocol::network::Headers;
use chromiumoxide::cdp::browser_protocol::network::SetExtraHttpHeadersParams;
use chromiumoxide::cdp::browser_protocol::network::SetUserAgentOverrideParams;
use chromiumoxide::error::CdpError;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

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
  tool_context: ToolContext,
  Parameters(EmulateRequest {
    page_id,
    network_conditions,
    cpu_throttling_rate,
    geolocation,
    user_agent,
    color_scheme,
    viewport,
    extra_http_headers,
  }): Parameters<EmulateRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  let geolocation = geolocation.as_deref().map(parse_geolocation).transpose()?;
  let viewport = viewport.as_deref().map(parse_viewport).transpose()?;
  let headers = extra_http_headers
    .as_deref()
    .map(parse_headers)
    .transpose()?;

  emulate_network(&page, network_conditions).await?;

  let cpu_throttling_rate = cpu_throttling_rate
    .filter(|rate| *rate != 1.0)
    .unwrap_or(1.0);
  page
    .execute(SetCpuThrottlingRateParams::new(cpu_throttling_rate))
    .await
    .map_err(internal)?;

  let geolocation = geolocation.map_or_else(
    || SetGeolocationOverrideParams::builder().build(),
    |location| {
      SetGeolocationOverrideParams::builder()
        .latitude(location.latitude)
        .longitude(location.longitude)
        .accuracy(0.0)
        .build()
    },
  );
  page.execute(geolocation).await.map_err(internal)?;

  let user_agent = match user_agent.filter(|value| !value.is_empty()) {
    Some(user_agent) => user_agent,
    None => tool_context
      .with_browser(async |browser| browser.user_agent().await)
      .await
      .map_err(internal)?,
  };
  page
    .execute(SetUserAgentOverrideParams::new(user_agent))
    .await
    .map_err(internal)?;

  let color_scheme = match color_scheme {
    Some(ColorScheme::Dark) => "dark",
    Some(ColorScheme::Light) => "light",
    Some(ColorScheme::Auto) | None => "",
  };
  let features = vec![MediaFeature::new("prefers-color-scheme", color_scheme)];
  page
    .execute(SetEmulatedMediaParams {
      media: None,
      features: Some(features),
    })
    .await
    .map_err(internal)?;

  match viewport {
    Some(viewport) => {
      let screen_orientation = if viewport.landscape {
        ScreenOrientation::new(ScreenOrientationType::LandscapePrimary, 90)
      } else {
        ScreenOrientation::new(ScreenOrientationType::PortraitPrimary, 0)
      };
      let params = SetDeviceMetricsOverrideParams::builder()
        .width(viewport.width)
        .height(viewport.height)
        .device_scale_factor(viewport.device_scale_factor)
        .mobile(viewport.mobile)
        .screen_orientation(screen_orientation)
        .build()
        .map_err(|error| ErrorData::internal_error(error, None))?;
      page.execute(params).await.map_err(internal)?;
      page
        .execute(SetTouchEmulationEnabledParams::new(viewport.touch))
        .await
        .map_err(internal)?;
    }
    None => {
      page
        .execute(ClearDeviceMetricsOverrideParams::default())
        .await
        .map_err(internal)?;
      page
        .execute(SetTouchEmulationEnabledParams::new(false))
        .await
        .map_err(internal)?;
    }
  }

  if let Some(headers) = headers {
    let headers = Headers::new(
      headers
        .into_iter()
        .map(|(name, value)| (name.to_lowercase(), serde_json::Value::String(value)))
        .collect::<serde_json::Map<String, serde_json::Value>>(),
    );
    page
      .execute(SetExtraHttpHeadersParams { headers })
      .await
      .map_err(internal)?;
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(
    "Emulation configured successfully",
  )]))
}

async fn emulate_network(
  page: &Page,
  network_conditions: Option<NetworkConditions>,
) -> Result<(), ErrorData> {
  let (offline, latency, download_throughput, upload_throughput) = match network_conditions {
    None => (false, 0.0, -1.0, -1.0),
    Some(NetworkConditions::Offline) => (true, 0.0, 0.0, 0.0),
    Some(NetworkConditions::Slow3g) => (false, 2000.0, 50000.0, 50000.0),
    Some(NetworkConditions::Fast3g | NetworkConditions::Slow4g) => {
      (false, 562.5, 180000.0, 84375.0)
    }
    Some(NetworkConditions::Fast4g) => (false, 165.0, 1012500.0, 168750.0),
  };

  #[allow(deprecated)]
  let params =
    EmulateNetworkConditionsParams::new(offline, latency, download_throughput, upload_throughput);
  page.execute(params).await.map_err(internal)?;

  Ok(())
}

struct Geolocation {
  latitude: f64,
  longitude: f64,
}

struct Viewport {
  width: i64,
  height: i64,
  device_scale_factor: f64,
  mobile: bool,
  touch: bool,
  landscape: bool,
}

fn parse_geolocation(value: &str) -> Result<Geolocation, ErrorData> {
  let (latitude, longitude) = value.split_once(',').ok_or_else(|| {
    invalid_value(
      value,
      "Expected format '<latitude>,<longitude>' with a latitude between -90 and 90 and a longitude between -180 and 180.",
    )
  })?;

  let latitude = latitude
    .trim()
    .parse::<f64>()
    .map_err(|_| invalid_value(value, "Latitude must be a number between -90 and 90."))?;
  let longitude = longitude
    .trim()
    .parse::<f64>()
    .map_err(|_| invalid_value(value, "Longitude must be a number between -180 and 180."))?;

  if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
    return Err(invalid_value(
      value,
      "Latitude must be a number between -90 and 90.",
    ));
  }
  if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
    return Err(invalid_value(
      value,
      "Longitude must be a number between -180 and 180.",
    ));
  }

  Ok(Geolocation {
    latitude,
    longitude,
  })
}

fn parse_viewport(value: &str) -> Result<Viewport, ErrorData> {
  let mut segments = value.split(',');
  let dimensions = segments.next().unwrap_or_default();
  let tags = segments.collect::<Vec<_>>();

  if let Some(tag) = tags
    .iter()
    .find(|tag| !matches!(tag.trim(), "mobile" | "touch" | "landscape"))
  {
    return Err(invalid_value(
      value,
      &format!("Unknown viewport tag \"{tag}\". Expected 'mobile', 'touch' or 'landscape'."),
    ));
  }

  let mut dimensions = dimensions.split('x');
  let width = dimensions.next().unwrap_or_default().trim();
  let height = dimensions.next().unwrap_or_default().trim();
  let dpr = dimensions.next().map(str::trim);

  if dimensions.next().is_some() {
    return Err(invalid_value(
      value,
      "Expected format '<width>x<height>x<devicePixelRatio>[,mobile][,touch][,landscape]'.",
    ));
  }

  let width = parse_dimension(width, "width", value)?;
  let height = parse_dimension(height, "height", value)?;
  let device_scale_factor = match dpr {
    Some(dpr) => {
      let dpr = dpr
        .parse::<f64>()
        .map_err(|_| invalid_value(value, "Device pixel ratio must be a positive number."))?;
      if !dpr.is_finite() || dpr <= 0.0 {
        return Err(invalid_value(
          value,
          "Device pixel ratio must be a positive number.",
        ));
      }
      dpr
    }
    None => 1.0,
  };

  Ok(Viewport {
    width,
    height,
    device_scale_factor,
    mobile: tags.iter().any(|tag| tag.trim() == "mobile"),
    touch: tags.iter().any(|tag| tag.trim() == "touch"),
    landscape: tags.iter().any(|tag| tag.trim() == "landscape"),
  })
}

fn parse_dimension(
  segment: &str,
  name: &str,
  value: &str,
) -> Result<i64, ErrorData> {
  let dimension = segment.parse::<f64>().map_err(|_| {
    invalid_value(
      value,
      &format!("Invalid viewport {name} \"{segment}\". Expected a positive integer."),
    )
  })?;

  if !dimension.is_finite() || dimension <= 0.0 || dimension.fract() != 0.0 {
    return Err(invalid_value(
      value,
      &format!("Invalid viewport {name} \"{segment}\". Expected a positive integer."),
    ));
  }

  Ok(dimension as i64)
}

fn parse_headers(value: &str) -> Result<BTreeMap<String, String>, ErrorData> {
  let json = if value.is_empty() { "{}" } else { value };
  let parsed = serde_json::from_str::<serde_json::Value>(json)
    .map_err(|error| invalid_value(json, &format!("Invalid JSON for headers: {error}")))?;

  let serde_json::Value::Object(headers) = parsed else {
    return Err(invalid_value(json, "Headers must be a JSON object."));
  };

  headers
    .into_iter()
    .map(|(name, value)| match value {
      serde_json::Value::String(value) => Ok((name, value)),
      other => Err(invalid_value(
        json,
        &format!("Expected value of header \"{name}\" to be a string, but found {other}."),
      )),
    })
    .collect()
}

fn invalid_value(
  value: &str,
  reason: &str,
) -> ErrorData {
  ErrorData::invalid_params(
    format!("Invalid emulation option \"{value}\": {reason}"),
    None,
  )
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
