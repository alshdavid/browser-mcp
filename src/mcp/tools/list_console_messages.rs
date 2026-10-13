use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ConsoleMessage;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum ConsoleMessageType {
  #[serde(rename = "log")]
  Log,
  #[serde(rename = "debug")]
  Debug,
  #[serde(rename = "info")]
  Info,
  #[serde(rename = "error")]
  Error,
  #[serde(rename = "warn")]
  Warn,
  #[serde(rename = "dir")]
  Dir,
  #[serde(rename = "dirxml")]
  Dirxml,
  #[serde(rename = "table")]
  Table,
  #[serde(rename = "trace")]
  Trace,
  #[serde(rename = "clear")]
  Clear,
  #[serde(rename = "startGroup")]
  StartGroup,
  #[serde(rename = "startGroupCollapsed")]
  StartGroupCollapsed,
  #[serde(rename = "endGroup")]
  EndGroup,
  #[serde(rename = "assert")]
  Assert,
  #[serde(rename = "profile")]
  Profile,
  #[serde(rename = "profileEnd")]
  ProfileEnd,
  #[serde(rename = "count")]
  Count,
  #[serde(rename = "timeEnd")]
  TimeEnd,
  #[serde(rename = "verbose")]
  Verbose,
  #[serde(rename = "issue")]
  Issue,
}

impl ConsoleMessageType {
  fn matches(
    self,
    message: &ConsoleMessage,
  ) -> bool {
    // CDP reports console.warn as `warning`, which is the spelling that has to
    // be compared against the tool's `warn` filter value.
    let reported = match message.message_type.as_str() {
      "warning" => "warn",
      other => other,
    };

    reported == self.as_str()
  }

  fn as_str(self) -> &'static str {
    match self {
      ConsoleMessageType::Log => "log",
      ConsoleMessageType::Debug => "debug",
      ConsoleMessageType::Info => "info",
      ConsoleMessageType::Error => "error",
      ConsoleMessageType::Warn => "warn",
      ConsoleMessageType::Dir => "dir",
      ConsoleMessageType::Dirxml => "dirxml",
      ConsoleMessageType::Table => "table",
      ConsoleMessageType::Trace => "trace",
      ConsoleMessageType::Clear => "clear",
      ConsoleMessageType::StartGroup => "startGroup",
      ConsoleMessageType::StartGroupCollapsed => "startGroupCollapsed",
      ConsoleMessageType::EndGroup => "endGroup",
      ConsoleMessageType::Assert => "assert",
      ConsoleMessageType::Profile => "profile",
      ConsoleMessageType::ProfileEnd => "profileEnd",
      ConsoleMessageType::Count => "count",
      ConsoleMessageType::TimeEnd => "timeEnd",
      ConsoleMessageType::Verbose => "verbose",
      ConsoleMessageType::Issue => "issue",
    }
  }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListConsoleMessagesRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Maximum number of messages to return. When omitted, returns all messages."
  )]
  pub page_size: Option<i64>,
  #[schemars(
    description = "Page number to return (0-based). When omitted, returns the first page."
  )]
  pub page_idx: Option<i64>,
  #[schemars(
    description = "Filter messages to only return messages of the specified resource types. When omitted or empty, returns all messages."
  )]
  pub types: Option<Vec<ConsoleMessageType>>,
  #[schemars(
    description = "Set to true to return the preserved messages over the last 3 navigations."
  )]
  pub include_preserved_messages: Option<bool>,
  #[schemars(
    description = "Set to true to include the stack trace for each message when available. Increases the response size."
  )]
  pub include_stack_traces: Option<bool>,
  #[schemars(
    description = "Filter messages to only return messages of the specified service worker."
  )]
  pub service_worker_id: Option<String>,
}

const DEFAULT_PAGE_SIZE: usize = 20;

pub async fn list_console_messages(
  tool_context: ToolContext,
  Parameters(ListConsoleMessagesRequest {
    page_id,
    page_size,
    page_idx,
    types,
    include_preserved_messages,
    include_stack_traces,
    service_worker_id,
  }): Parameters<ListConsoleMessagesRequest>,
) -> Result<CallToolResult, ErrorData> {
  tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  if let Some(service_worker_id) = service_worker_id {
    return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
      "## Console messages\n<no console messages found for service worker {service_worker_id}>"
    ))]));
  }

  let mut messages = tool_context
    .console_messages(page_id, include_preserved_messages.unwrap_or(false))
    .await;

  if let Some(types) = types.as_ref().filter(|types| !types.is_empty()) {
    messages.retain(|message| types.iter().any(|kind| kind.matches(message)));
  }

  let grouped = group_consecutive(messages);
  let pagination = paginate(&grouped, page_size, page_idx);

  let mut lines = vec!["## Console messages".to_owned()];

  if grouped.is_empty() {
    lines.push("<no console messages found>".to_owned());
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      lines.join("\n"),
    )]));
  }

  lines.extend(pagination.info.clone());
  lines.extend(
    pagination
      .items
      .iter()
      .map(|message| format_message(message, include_stack_traces.unwrap_or(false))),
  );

  if include_stack_traces.unwrap_or(false)
    && pagination
      .items
      .iter()
      .any(|message| !message.message.frames.is_empty())
  {
    lines.push("Note: stack trace line and column numbers use 1-based indexing".to_owned());
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(
    lines.join("\n"),
  )]))
}

#[derive(Debug, Clone)]
struct GroupedMessage {
  message: ConsoleMessage,
  count: usize,
}

fn group_consecutive(messages: Vec<ConsoleMessage>) -> Vec<GroupedMessage> {
  let mut grouped: Vec<GroupedMessage> = Vec::new();

  for message in messages {
    let repeated = grouped.last().is_some_and(|previous| {
      previous.message.message_type == message.message_type
        && previous.message.text == message.text
        && previous.message.args == message.args
        && previous.message.preserved == message.preserved
    });

    if repeated {
      if let Some(previous) = grouped.last_mut() {
        previous.count += 1;
      }
    } else {
      grouped.push(GroupedMessage { message, count: 1 });
    }
  }

  grouped
}

struct Pagination {
  items: Vec<GroupedMessage>,
  info: Vec<String>,
}

fn paginate(
  grouped: &[GroupedMessage],
  page_size: Option<i64>,
  page_idx: Option<i64>,
) -> Pagination {
  let Some(page_size) = page_size.or(page_idx.map(|_| DEFAULT_PAGE_SIZE as i64)) else {
    return Pagination {
      items: grouped.to_vec(),
      info: vec![showing(0, grouped.len(), 0, 1, grouped.len())],
    };
  };

  let page_size = page_size.max(1) as usize;
  let total_pages = grouped.len().div_ceil(page_size).max(1);
  let requested = page_idx.unwrap_or(0);
  let invalid_page = requested < 0 || requested as usize >= total_pages;
  let current_page = if invalid_page { 0 } else { requested as usize };

  let start = current_page * page_size;
  let end = (start + page_size).min(grouped.len());
  let items = grouped[start.min(grouped.len())..end].to_vec();

  let mut info = Vec::new();
  if invalid_page {
    info.push("Invalid page number provided. Showing first page.".to_owned());
  }

  info.push(showing(
    start,
    end,
    current_page,
    total_pages,
    grouped.len(),
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

fn format_message(
  grouped: &GroupedMessage,
  include_stack_traces: bool,
) -> String {
  let message = &grouped.message;
  let count_suffix = if grouped.count > 1 {
    format!(" [{} times]", grouped.count)
  } else {
    String::new()
  };
  let preserved_suffix = if message.preserved {
    " [preserved]"
  } else {
    ""
  };

  let mut line = format!(
    "msgid={} [{}] {}{} ({} args){}",
    message.id,
    message.message_type,
    message.text,
    preserved_suffix,
    message.args.len(),
    count_suffix
  );

  if include_stack_traces && !message.frames.is_empty() {
    for frame in &message.frames {
      line.push_str(&format!(
        "\n  at {} ({}:{}:{})",
        if frame.function_name.is_empty() {
          "<anonymous>"
        } else {
          &frame.function_name
        },
        frame.url,
        frame.line_number,
        frame.column_number
      ));
    }
  }

  line
}
