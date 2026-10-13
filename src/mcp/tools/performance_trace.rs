use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::io::CloseParams;
use chromiumoxide::cdp::browser_protocol::io::ReadParams;
use chromiumoxide::cdp::browser_protocol::io::StreamHandle;
use chromiumoxide::cdp::browser_protocol::tracing::EndParams;
use chromiumoxide::cdp::browser_protocol::tracing::EventTracingComplete;
use chromiumoxide::cdp::browser_protocol::tracing::StartParams;
use chromiumoxide::cdp::browser_protocol::tracing::StartTransferMode;
use chromiumoxide::cdp::browser_protocol::tracing::StreamFormat;
use chromiumoxide::cdp::browser_protocol::tracing::TraceConfig;
use chromiumoxide::error::CdpError;
use futures_util::StreamExt as _;
use rmcp::ErrorData;

use crate::mcp::tool_context::RecordedTrace;
use crate::mcp::tool_context::TraceInsight;
use crate::mcp::tool_context::TraceInsightSet;

pub const TRACE_TIMEOUT: Duration = Duration::from_secs(30);

// Mirrors DevTools' Trace.Types.Events.DefaultCategories, which is what
// DevTools.TracingDefaultCategories re-exports upstream. The leading '-*'
// excludes every other category, exactly as upstream does.
const DEFAULT_CATEGORIES: &[&str] = &[
  "blink.console",
  "blink.user_timing",
  "loading",
  "devtools.timeline",
  "disabled-by-default-devtools.target-rundown",
  "disabled-by-default-devtools.timeline.frame",
  "disabled-by-default-devtools.timeline.stack",
  "disabled-by-default-devtools.timeline",
  "disabled-by-default-devtools.v8-source-rundown-sources",
  "disabled-by-default-devtools.v8-source-rundown",
  "disabled-by-default-layout_shift.debug",
  "disabled-by-default-v8.inspector",
  "disabled-by-default-v8.cpu_profiler.hires",
  "disabled-by-default-lighthouse",
  "v8.execute",
  "v8",
  "cppgc",
  "navigation,rail",
];

// DevTools enables JS sampling and screenshots on top of the default set.
const JS_SAMPLING_CATEGORY: &str = "disabled-by-default-v8.cpu_profiler";
const SCREENSHOT_CATEGORY: &str = "disabled-by-default-devtools.screenshot";

pub fn categories() -> Vec<String> {
  let mut categories = Vec::with_capacity(DEFAULT_CATEGORIES.len() + 3);
  categories.push("-*".to_owned());
  categories.extend(
    DEFAULT_CATEGORIES
      .iter()
      .map(|category| (*category).to_owned()),
  );
  categories.push(JS_SAMPLING_CATEGORY.to_owned());
  categories.push(SCREENSHOT_CATEGORY.to_owned());
  categories
}

pub async fn start(
  page: &Page,
  buffer_size_in_kb: f64,
) -> Result<(), CdpError> {
  let config = TraceConfig {
    included_categories: Some(categories()),
    enable_sampling: Some(true),
    trace_buffer_size_in_kb: Some(buffer_size_in_kb),
    ..TraceConfig::default()
  };

  // ReportEvents, the CDP default, streams every trace event to the client as it
  // is recorded. ReturnAsStream collects them in the browser and hands back a
  // single stream handle on Tracing.tracingComplete instead.
  let params = StartParams {
    transfer_mode: Some(StartTransferMode::ReturnAsStream),
    stream_format: Some(StreamFormat::Json),
    trace_config: Some(config),
    ..StartParams::default()
  };

  page.execute(params).await.map(|_| ())
}

pub async fn stop(page: &Page) -> Result<Vec<u8>, ErrorData> {
  // Tracing.end returns no result in this CDP revision; the payload arrives on
  // the tracingComplete event, so the listener must exist before end is sent.
  let mut complete = page
    .event_listener::<EventTracingComplete>()
    .await
    .map_err(internal)?;

  page.execute(EndParams::default()).await.map_err(internal)?;

  let event = tokio::time::timeout(TRACE_TIMEOUT, complete.next())
    .await
    .map_err(|_| {
      ErrorData::internal_error(
        "Timed out waiting for the trace to be written.".to_owned(),
        None,
      )
    })?
    .ok_or_else(|| {
      ErrorData::internal_error(
        "The trace stream closed before it completed.".to_owned(),
        None,
      )
    })?;

  let Some(handle) = event.stream.clone() else {
    return Err(ErrorData::internal_error(
      "The browser did not return a trace stream.".to_owned(),
      None,
    ));
  };

  let raw = read_stream(page, &handle).await;
  let _ = page.execute(CloseParams::new(handle)).await;
  raw
}

async fn read_stream(
  page: &Page,
  handle: &StreamHandle,
) -> Result<Vec<u8>, ErrorData> {
  let mut raw = Vec::new();

  loop {
    let chunk = page
      .execute(
        ReadParams::builder()
          .handle(handle.clone())
          .build()
          .map_err(|error| ErrorData::internal_error(error, None))?,
      )
      .await
      .map_err(internal)?;

    let data = decode_chunk(&chunk.result.data, chunk.result.base64_encoded)?;
    raw.extend_from_slice(&data);

    if chunk.result.eof {
      return Ok(raw);
    }
  }
}

fn decode_chunk(
  data: &str,
  base64_encoded: Option<bool>,
) -> Result<Vec<u8>, ErrorData> {
  if base64_encoded.unwrap_or(false) {
    use base64::Engine as _;
    return base64::engine::general_purpose::STANDARD
      .decode(data)
      .map_err(|error| ErrorData::internal_error(error.to_string(), None));
  }

  Ok(data.as_bytes().to_vec())
}

pub fn parse_recording(
  url: String,
  raw: Vec<u8>,
) -> RecordedTrace {
  let events = parse_events(&raw);
  let (event_count, duration_seconds) = measure(&events);
  let insights = insights(&events);

  RecordedTrace {
    url,
    raw,
    events,
    event_count,
    duration_seconds,
    insights,
  }
}

pub fn describe(trace: &RecordedTrace) -> String {
  let mut lines = vec![format!(
    "Recorded {} trace events spanning {:.2}s on {}.",
    trace.event_count, trace.duration_seconds, trace.url
  )];

  if trace.raw.is_empty() {
    lines.push("The browser returned an empty trace.".to_owned());
  }

  for set in &trace.insights {
    let lines_for_set = describe_insight_set(trace, &set.insight_set_id);
    lines.extend(lines_for_set);
  }

  lines.join("\n")
}

pub fn describe_insight_set(
  trace: &RecordedTrace,
  insight_set_id: &str,
) -> Vec<String> {
  let Some(set) = insight_set(trace, insight_set_id) else {
    return vec![format!(
      "No Performance Insights for the given insight set id \"{insight_set_id}\". Only use ids given in the \"Available insight sets\" list."
    )];
  };

  let mut lines = vec![format!(
    "Insight set \"{}\" ({}):",
    set.insight_set_id, set.url
  )];

  let mut insights = set.insights.values().collect::<Vec<_>>();
  insights.sort_by(|left, right| left.name.cmp(&right.name));

  for insight in insights {
    lines.push(format!("  {}", format_insight(insight)));
  }

  lines
}

pub fn insight_set(
  trace: &RecordedTrace,
  insight_set_id: &str,
) -> Option<TraceInsightSet> {
  trace
    .insights
    .iter()
    .find(|set| set.insight_set_id == insight_set_id)
    .cloned()
}

pub fn event_names(trace: &RecordedTrace) -> Vec<String> {
  let mut names = trace
    .events
    .iter()
    .filter_map(|event| event.get("name").and_then(serde_json::Value::as_str))
    .map(str::to_owned)
    .collect::<Vec<_>>();
  names.sort();
  names.dedup();
  names
}

pub fn format_insight(insight: &TraceInsight) -> String {
  let metrics = insight
    .metrics
    .iter()
    .map(|(name, value)| format!("{name}={value:.2}"))
    .collect::<Vec<_>>()
    .join(", ");

  if insight.description.is_empty() {
    return format!("{}: {metrics}", insight.title);
  }

  format!("{}: {metrics} ({})", insight.title, insight.description)
}

fn parse_events(raw: &[u8]) -> Vec<serde_json::Value> {
  let Ok(value) = serde_json::from_slice::<serde_json::Value>(raw) else {
    return Vec::new();
  };

  match value {
    serde_json::Value::Array(events) => events,
    serde_json::Value::Object(mut object) => match object.remove("traceEvents") {
      Some(serde_json::Value::Array(events)) => events,
      _ => Vec::new(),
    },
    _ => Vec::new(),
  }
}

fn measure(events: &[serde_json::Value]) -> (usize, f64) {
  let timestamps = events
    .iter()
    .filter_map(|event| event.get("ts").and_then(serde_json::Value::as_f64));

  let mut minimum = f64::INFINITY;
  let mut maximum = f64::NEG_INFINITY;

  for timestamp in timestamps {
    minimum = minimum.min(timestamp);
    maximum = maximum.max(timestamp);
  }

  if !minimum.is_finite() || !maximum.is_finite() {
    return (events.len(), 0.0);
  }

  (events.len(), (maximum - minimum) / 1_000_000.0)
}

fn insights(events: &[serde_json::Value]) -> Vec<TraceInsightSet> {
  let Some(main_frame) = main_frame_url(events) else {
    return Vec::new();
  };

  let Some(navigation_id) = navigation_id(events) else {
    return Vec::new();
  };

  let mut set = TraceInsightSet {
    insight_set_id: navigation_id,
    url: main_frame,
    insights: std::collections::HashMap::new(),
  };

  for (name, title, timestamp_name) in [
    ("FCP", "First Contentful Paint", "firstContentfulPaint"),
    ("LCP", "Largest Contentful Paint", "largestContentfulPaint"),
  ] {
    let Some(value) = paint_mark(events, timestamp_name) else {
      continue;
    };
    set.insights.insert(
      name.to_owned(),
      TraceInsight {
        name: name.to_owned(),
        title: title.to_owned(),
        description: String::new(),
        metrics: vec![("timestamp".to_owned(), value)],
      },
    );
  }

  if let Some(duration) = longest_task(events) {
    set.insights.insert(
      "LongTasks".to_owned(),
      TraceInsight {
        name: "LongTasks".to_owned(),
        title: "Long Tasks".to_owned(),
        description: String::new(),
        metrics: vec![("longestTaskDuration".to_owned(), duration)],
      },
    );
  }

  if set.insights.is_empty() {
    return Vec::new();
  }

  vec![set]
}

fn main_frame_url(events: &[serde_json::Value]) -> Option<String> {
  events.iter().find_map(|event| {
    if event.get("name").and_then(serde_json::Value::as_str) != Some("TracingStartedInBrowser") {
      return None;
    }

    event
      .get("args")?
      .get("data")?
      .get("frames")?
      .as_array()?
      .first()?
      .get("url")?
      .as_str()
      .map(str::to_owned)
  })
}

fn navigation_id(events: &[serde_json::Value]) -> Option<String> {
  events.iter().find_map(|event| {
    if event.get("name").and_then(serde_json::Value::as_str) != Some("navigationStart") {
      return None;
    }

    event
      .get("args")?
      .get("data")?
      .get("navigationId")?
      .as_str()
      .map(str::to_owned)
  })
}

fn paint_mark(
  events: &[serde_json::Value],
  name: &str,
) -> Option<f64> {
  events.iter().find_map(|event| {
    if event.get("name").and_then(serde_json::Value::as_str) != Some(name) {
      return None;
    }

    event.get("ts").and_then(serde_json::Value::as_f64)
  })
}

fn longest_task(events: &[serde_json::Value]) -> Option<f64> {
  events
    .iter()
    .filter(|event| {
      matches!(
        event.get("name").and_then(serde_json::Value::as_str),
        Some("RunTask" | "ThreadControllerImpl::RunTask")
      )
    })
    .filter_map(|event| event.get("dur").and_then(serde_json::Value::as_f64))
    .fold(None, |longest: Option<f64>, duration| {
      Some(longest.map_or(duration, |longest| longest.max(duration)))
    })
    .map(|duration| duration / 1_000.0)
}

fn internal(error: CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
