use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ConsoleFrame;
use crate::mcp::tool_context::ConsoleMessage;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetConsoleMessageRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The msgid of a console message on the page from the listed console messages"
  )]
  pub msgid: f64,
}

const STACK_TRACE_MAX_LINES: usize = 50;

pub async fn get_console_message(
  tool_context: ToolContext,
  Parameters(GetConsoleMessageRequest { page_id, msgid }): Parameters<GetConsoleMessageRequest>,
) -> Result<CallToolResult, ErrorData> {
  let _ = tool_context.require_page(page_id).await?;

  let known = tool_context.console_messages(page_id, true).await;
  let msgnum = known
    .iter()
    .find(|message| message.id as f64 == msgid)
    .map(|message| message.id);

  let message = match msgnum {
    Some(msgnum) => tool_context.console_message(page_id, msgnum).await,
    None => None,
  }
  .ok_or_else(|| {
    ErrorData::invalid_params(
      format!("No console message found with msgid {msgid} on page {page_id}."),
      None,
    )
  })?;

  Ok(CallToolResult::success(vec![ContentBlock::text(detailed(
    &message,
  ))]))
}

fn detailed(message: &ConsoleMessage) -> String {
  let text = if message.text.is_empty() {
    String::new()
  } else {
    message.text.clone()
  };

  let args = arguments(message, text.is_empty());

  let mut parts = vec![
    format!("ID: {}", message.id),
    format!("Message: {}> {}", message.message_type, text),
  ];

  if !args.is_empty() {
    let mut block = vec!["### Arguments".to_owned()];
    block.extend(
      args
        .iter()
        .enumerate()
        .map(|(index, arg)| format!("Arg #{index}: {arg}")),
    );
    parts.push(block.join("\n"));
  }

  if !message.frames.is_empty() {
    parts.push("### Stack trace".to_owned());
    parts.push(stack_trace(&message.frames));
  }

  parts.retain(|part| !part.is_empty());
  parts.join("\n")
}

fn arguments(
  message: &ConsoleMessage,
  shift: bool,
) -> Vec<String> {
  let mut args = message.args.clone();

  if shift && !args.is_empty() {
    args.remove(0);
  }

  args
}

fn stack_trace(frames: &[ConsoleFrame]) -> String {
  let mut lines: Vec<String> = frames
    .iter()
    .take(STACK_TRACE_MAX_LINES)
    .map(|frame| {
      format!(
        "at {} ({}:{}:{})",
        if frame.function_name.is_empty() {
          "<anonymous>"
        } else {
          &frame.function_name
        },
        frame.url,
        frame.line_number,
        frame.column_number
      )
    })
    .collect();

  let remaining = frames.len().saturating_sub(STACK_TRACE_MAX_LINES);
  if remaining > 0 {
    lines.push(format!("... and {remaining} more frames"));
  }

  lines.push("Note: line and column numbers use 1-based indexing".to_owned());
  lines.join("\n")
}
