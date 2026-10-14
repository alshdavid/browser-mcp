use std::path::Path;
use std::path::PathBuf;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::ResolveNodeParams;
use chromiumoxide::cdp::js_protocol::runtime::CallArgument;
use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::cdp::js_protocol::runtime::ExceptionDetails;
use chromiumoxide::cdp::js_protocol::runtime::RemoteObject;
use chromiumoxide::cdp::js_protocol::runtime::RemoteObjectId;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

const WRAPPER_FUNCTION: &str =
  "async function (...args) { return JSON.stringify(await this(...args)); }";

const STABLE_DOM_EXPRESSION: &str = r#"new Promise((resolve) => {
  const timeout = 100;
  const cap = 3000;
  let settled = false;
  let quiet;
  const finish = () => {
    if (settled) {
      return;
    }
    settled = true;
    clearTimeout(quiet);
    clearTimeout(hardCap);
    observer.disconnect();
    resolve();
  };
  const hardCap = setTimeout(finish, cap);
  const observer = new MutationObserver(() => {
    clearTimeout(quiet);
    quiet = setTimeout(finish, timeout);
  });
  quiet = setTimeout(finish, timeout);
  observer.observe(document.body, { childList: true, subtree: true, attributes: true });
})"#;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateScriptRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "A JavaScript function declaration to be executed by the tool in the target page.\nExample without arguments: `() => document.title` or `async () => await fetch(\"example.com\")`.\nExample with arguments: `(el) => el.innerText`\n"
  )]
  pub function: String,
  #[schemars(description = "An optional list of arguments to pass to the function.")]
  pub args: Option<Vec<String>>,
  #[schemars(
    description = "The absolute or relative path to a file to save the script output to. If omitted, the output is returned inline."
  )]
  pub file_path: Option<String>,
  #[schemars(
    description = "Handle dialogs while execution. \"accept\", \"dismiss\", or string for response of window.prompt. Defaults to accept."
  )]
  pub dialog_action: Option<String>,
  #[schemars(
    description = "Whether to wait for the DOM to settle. Pass false if the script only reads data. Defaults to true."
  )]
  pub wait_for_stable_dom: Option<bool>,
}

pub async fn evaluate_script(
  tool_context: ToolContext,
  Parameters(EvaluateScriptRequest {
    page_id,
    function,
    args,
    file_path,
    dialog_action,
    wait_for_stable_dom,
  }): Parameters<EvaluateScriptRequest>,
) -> Result<CallToolResult, ErrorData> {
  // Dialog handling is intentionally unsupported: CDP exposes no dialog-query API.
  let _ = dialog_action;

  let page = tool_context.require_page(page_id).await?;

  let object_ids = resolve_arguments(&tool_context, page_id, &page, args.as_deref()).await?;

  let result = evaluate(&page, &function, &object_ids).await?;

  if wait_for_stable_dom.unwrap_or(true) {
    let _ = page
      .execute(
        EvaluateParams::builder()
          .expression(STABLE_DOM_EXPRESSION)
          .return_by_value(true)
          .await_promise(true)
          .build()
          .map_err(|error| ErrorData::internal_error(error.to_string(), None))?,
      )
      .await;
  }

  match file_path {
    Some(file_path) => {
      let path = with_json_extension(&file_path);
      let filename = path.to_string_lossy().into_owned();
      write_output(&path, &result)?;
      Ok(CallToolResult::success(vec![ContentBlock::text(format!(
        "Script ran on page. Output saved to {filename}."
      ))]))
    }
    None => {
      let text = format!("Script ran on page and returned:\n```json\n{result}\n```");
      Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
  }
}

async fn resolve_arguments(
  tool_context: &ToolContext,
  page_id: f64,
  page: &Page,
  args: Option<&[String]>,
) -> Result<Vec<RemoteObjectId>, ErrorData> {
  let args = args.unwrap_or_default();
  if args.is_empty() {
    return Ok(Vec::new());
  }

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  let mut object_ids = Vec::with_capacity(args.len());
  for uid in args {
    let backend_node_id = tool_context
      .backend_node_for_uid(page_id, uid)
      .await
      .ok_or_else(|| unknown_uid(uid, page_id))?;

    let object_id = page
      .execute(
        ResolveNodeParams::builder()
          .backend_node_id(BackendNodeId::new(backend_node_id))
          .build(),
      )
      .await
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?
      .result
      .object
      .object_id
      .ok_or_else(|| unknown_uid(uid, page_id))?;

    object_ids.push(object_id);
  }

  Ok(object_ids)
}

fn unknown_uid(
  uid: &str,
  page_id: f64,
) -> ErrorData {
  ErrorData::invalid_params(
    format!("Element uid \"{uid}\" not found on page {page_id}."),
    None,
  )
}

async fn evaluate(
  page: &Page,
  function: &str,
  object_ids: &[RemoteObjectId],
) -> Result<String, ErrorData> {
  let source = function
    .trim_end()
    .strip_suffix(';')
    .unwrap_or(function.trim_end());

  let compiled = page
    .execute(
      EvaluateParams::builder()
        .expression(format!("(\n{source}\n)"))
        .return_by_value(false)
        .build()
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))?,
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  if let Some(exception) = compiled.result.exception_details.as_ref() {
    return Err(ErrorData::internal_error(exception_reason(exception), None));
  }

  let function_id = compiled.result.result.object_id.ok_or_else(|| {
    ErrorData::internal_error("Failed to compile the provided function".to_owned(), None)
  })?;

  let mut builder = CallFunctionOnParams::builder()
    .function_declaration(WRAPPER_FUNCTION)
    .object_id(function_id)
    .await_promise(true);

  if !object_ids.is_empty() {
    builder = builder.arguments(
      object_ids
        .iter()
        .cloned()
        .map(|object_id| CallArgument::builder().object_id(object_id).build())
        .collect::<Vec<_>>(),
    );
  }

  let called = page
    .execute(
      builder
        .build()
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))?,
    )
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

  if let Some(exception) = called.result.exception_details.as_ref() {
    return Err(ErrorData::internal_error(exception_reason(exception), None));
  }

  Ok(serialize_result(&called.result.result))
}

fn serialize_result(result: &RemoteObject) -> String {
  match &result.value {
    Some(serde_json::Value::String(text)) => text.clone(),
    Some(value) => value.to_string(),
    None => match &result.unserializable_value {
      Some(value) => value.inner().clone(),
      None => "undefined".to_owned(),
    },
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

fn with_json_extension(file_path: &str) -> PathBuf {
  let path = Path::new(file_path);
  let mut filename = path.file_name().map(PathBuf::from).unwrap_or_default();
  filename.set_extension("json");

  match path.parent() {
    Some(parent) => parent.join(filename),
    None => filename,
  }
}

fn write_output(
  path: &Path,
  contents: &str,
) -> Result<(), ErrorData> {
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent)
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  std::fs::write(path, contents).map_err(|error| ErrorData::internal_error(error.to_string(), None))
}
