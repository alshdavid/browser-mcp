use std::time::Duration;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::DescribeNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::ResolveNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::ScrollIntoViewIfNeededParams;
use chromiumoxide::cdp::js_protocol::runtime::CallArgument;
use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
use chromiumoxide::cdp::js_protocol::runtime::ExceptionDetails;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::take_snapshot::accessibility_tree;
use crate::mcp::tools::take_snapshot::assign_uids;
use crate::mcp::tools::take_snapshot::format_tree;

pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const TIMEOUT_PER_CHAR: Duration = Duration::from_millis(10);

const FILL_FUNCTION: &str = r#"function (value, asSelect) {
  const element = this;
  const tag = element.tagName;
  if (tag === 'SELECT' || asSelect) {
    if (tag !== 'SELECT') {
      throw new Error('Element is not a <select> element');
    }
    let matched = false;
    for (const option of element.options) {
      if (option.textContent.trim() === value) {
        element.value = option.value;
        matched = true;
        break;
      }
    }
    if (!matched) {
      throw new Error('Could not find option with text "' + value + '"');
    }
  } else {
    const type = (element.getAttribute('type') || '').toLowerCase();
    const role = element.getAttribute('role');
    const toggle = (tag === 'INPUT' && (type === 'checkbox' || type === 'radio'))
      || role === 'checkbox' || role === 'radio' || role === 'switch';
    if (toggle) {
      if (value !== 'true' && value !== 'false') {
        throw new Error('Checkboxes, radio boxes and toggles require "true" or "false" value, but ' + value + ' was used');
      }
      if (element.tagName === 'INPUT') {
        element.checked = value === 'true';
      } else {
        element.setAttribute('aria-checked', value);
      }
    } else {
      if (tag !== 'INPUT' && tag !== 'TEXTAREA' && tag !== 'SELECT') {
        throw new Error('Element is not an <input>, <textarea> or <select> element');
      }
      element.focus();
      if (element.select) {
        element.select();
      }
      element.value = value;
    }
  }
  element.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
  element.dispatchEvent(new Event('change', { bubbles: true }));
  return 'ok';
}"#;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The uid of an element on the page from the page content snapshot")]
  pub uid: String,
  #[schemars(
    description = "The value to fill in. \"true\" or \"false\" for checkboxes and toggles, \"true\" for radio buttons."
  )]
  pub value: String,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn fill(
  tool_context: ToolContext,
  Parameters(FillRequest {
    page_id,
    uid,
    value,
    include_snapshot,
  }): Parameters<FillRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  let node = tool_context
    .node_for_uid(page_id, &uid)
    .await
    .ok_or_else(|| unknown_uid(&uid, page_id))?;

  let backend_node_id = node
    .backend_node_id
    .filter(|backend_node_id| *backend_node_id != 0)
    .ok_or_else(|| unknown_uid(&uid, page_id))?;

  let is_combobox_select =
    node.role == "combobox" && has_option_children(&page, backend_node_id).await?;

  let length = u32::try_from(value.chars().count()).unwrap_or(u32::MAX);
  let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT + TIMEOUT_PER_CHAR * length;

  fill_element(
    &page,
    backend_node_id,
    &value,
    deadline,
    &uid,
    is_combobox_select,
  )
  .await?;

  let mut text = "Successfully filled out the element".to_owned();
  if include_snapshot.unwrap_or(false) {
    text.push_str("\n## Latest page snapshot\n");
    text.push_str(&snapshot(&tool_context, page_id, &page).await?);
  }

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

pub(crate) fn unknown_uid(
  uid: &str,
  page_id: f64,
) -> ErrorData {
  ErrorData::invalid_params(
    format!("Element uid \"{uid}\" not found on page {page_id}."),
    None,
  )
}

pub(crate) async fn has_option_children(
  page: &Page,
  backend_node_id: i64,
) -> Result<bool, ErrorData> {
  let described = page
    .execute(
      DescribeNodeParams::builder()
        .backend_node_id(BackendNodeId::new(backend_node_id))
        .depth(-1)
        .build(),
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  let mut stack = described.result.node.children.unwrap_or_default();
  while let Some(node) = stack.pop() {
    if node.node_name == "OPTION" {
      return Ok(true);
    }
    if let Some(children) = node.children {
      stack.extend(children);
    }
  }

  Ok(false)
}

pub(crate) async fn fill_element(
  page: &Page,
  backend_node_id: i64,
  value: &str,
  deadline: tokio::time::Instant,
  uid: &str,
  is_combobox_select: bool,
) -> Result<(), ErrorData> {
  loop {
    match attempt(page, backend_node_id, value, is_combobox_select).await {
      Ok(()) => return Ok(()),
      Err(failure) if failure.vanished => {
        return Err(ErrorData::invalid_params(
          format!("Element with uid {uid} no longer exists on the page."),
          None,
        ));
      }
      Err(failure) if failure.retryable && tokio::time::Instant::now() < deadline => {
        let next = (tokio::time::Instant::now() + POLL_INTERVAL).min(deadline);
        tokio::time::sleep_until(next).await;
      }
      Err(failure) => {
        return Err(ErrorData::internal_error(
          format!(
            "Failed to interact with the element with uid {uid}. {}",
            failure.reason
          ),
          None,
        ));
      }
    }
  }
}

struct Failure {
  reason: String,
  retryable: bool,
  vanished: bool,
}

impl Failure {
  fn fatal(reason: String) -> Self {
    Self {
      reason,
      retryable: false,
      vanished: false,
    }
  }

  fn classify(reason: String) -> Self {
    let vanished = reason.contains("No node with given id found")
      || reason.contains("Could not find node")
      || reason.contains("does not belong to the document");

    Self {
      reason,
      retryable: vanished,
      vanished,
    }
  }
}

async fn attempt(
  page: &Page,
  backend_node_id: i64,
  value: &str,
  is_combobox_select: bool,
) -> Result<(), Failure> {
  let backend_node_id = BackendNodeId::new(backend_node_id);

  page
    .execute(
      DescribeNodeParams::builder()
        .backend_node_id(backend_node_id)
        .depth(0)
        .build(),
    )
    .await
    .map_err(|error| Failure::classify(error.to_string()))?;

  page
    .execute(
      ScrollIntoViewIfNeededParams::builder()
        .backend_node_id(backend_node_id)
        .build(),
    )
    .await
    .map_err(|error| Failure::classify(error.to_string()))?;

  fill_object(page, backend_node_id, value, is_combobox_select)
    .await
    .map_err(Failure::fatal)
}

async fn fill_object(
  page: &Page,
  backend_node_id: BackendNodeId,
  value: &str,
  is_combobox_select: bool,
) -> Result<(), String> {
  let object_id = page
    .execute(
      ResolveNodeParams::builder()
        .backend_node_id(backend_node_id)
        .build(),
    )
    .await
    .map_err(|error| error.to_string())?
    .result
    .object
    .object_id
    .ok_or_else(|| "Could not find node".to_owned())?;

  let response = page
    .execute(
      CallFunctionOnParams::builder()
        .function_declaration(FILL_FUNCTION)
        .object_id(object_id)
        .arguments(vec![
          CallArgument::builder()
            .value(serde_json::Value::String(value.to_owned()))
            .build(),
          CallArgument::builder()
            .value(serde_json::Value::Bool(is_combobox_select))
            .build(),
        ])
        .return_by_value(true)
        .build()
        .map_err(|error| error.to_string())?,
    )
    .await
    .map_err(|error| error.to_string())?;

  match response.result.exception_details.as_ref() {
    Some(exception) => Err(exception_reason(exception)),
    None => Ok(()),
  }
}

fn exception_reason(exception: &ExceptionDetails) -> String {
  exception
    .exception
    .as_ref()
    .and_then(|object| object.description.as_ref())
    .cloned()
    .unwrap_or_else(|| exception.text.clone())
}

pub(crate) async fn snapshot(
  tool_context: &ToolContext,
  page_id: f64,
  page: &Page,
) -> Result<String, ErrorData> {
  let nodes = accessibility_tree(page)
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  let (snapshot_id, previous_uids) = tool_context.begin_snapshot(page_id).await;
  let (root, snapshot) = assign_uids(&nodes, snapshot_id, false, previous_uids);
  let text = format_tree(root.as_ref(), false);
  tool_context.store_snapshot(page_id, snapshot).await;

  Ok(text)
}
