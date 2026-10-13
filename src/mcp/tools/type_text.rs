use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventParams;
use chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventType;
use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use chromiumoxide::keys::KeyDefinition;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::keys::dispatch;
use crate::mcp::tools::keys::parse_key;
use crate::mcp::tools::keys::resolve_key;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TypeTextRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "The text to type")]
  pub text: String,
  #[schemars(
    description = "Optional key to press after typing. E.g., \"Enter\", \"Tab\", \"Escape\""
  )]
  pub submit_key: Option<String>,
}

pub async fn type_text(
  tool_context: ToolContext,
  Parameters(TypeTextRequest {
    page_id,
    text,
    submit_key,
  }): Parameters<TypeTextRequest>,
) -> Result<CallToolResult, ErrorData> {
  let submit_key = submit_key.filter(|key| !key.is_empty());

  let submit_tokens = match submit_key.as_deref() {
    Some(key) => Some(parse_key(key).map_err(|error| ErrorData::invalid_params(error, None))?),
    None => None,
  };

  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  type_characters(&page, &text)
    .await
    .map_err(|error| ErrorData::internal_error(error, None))?;

  if let Some(tokens) = submit_tokens {
    dispatch(&page, &tokens)
      .await
      .map_err(|error| ErrorData::internal_error(error, None))?;
  }

  let suffix = submit_key.map_or_else(String::new, |key| format!(" + {key}"));
  Ok(CallToolResult::success(vec![ContentBlock::text(format!(
    "Typed text \"{text}{suffix}\""
  ))]))
}

async fn type_characters(
  page: &Page,
  text: &str,
) -> Result<(), String> {
  for character in text.chars() {
    let mut buffer = [0_u8; 4];
    let character = character.encode_utf8(&mut buffer);

    match resolve_key(character) {
      Some(definition) => {
        press_character(page, definition).await?;
      }
      None => {
        page
          .execute(
            InsertTextParams::builder()
              .text(character)
              .build()
              .map_err(|error| error.to_string())?,
          )
          .await
          .map_err(|error| error.to_string())?;
      }
    }
  }

  Ok(())
}

async fn press_character(
  page: &Page,
  definition: &KeyDefinition,
) -> Result<(), String> {
  let text = definition
    .text
    .or(Some(definition.key))
    .filter(|text| text.len() <= 1);

  let event_type = if text.is_some() {
    DispatchKeyEventType::KeyDown
  } else {
    DispatchKeyEventType::RawKeyDown
  };

  let mut builder = DispatchKeyEventParams::builder()
    .r#type(event_type)
    .key(definition.key)
    .code(definition.code)
    .windows_virtual_key_code(definition.key_code)
    .native_virtual_key_code(definition.key_code);

  if let Some(text) = text {
    builder = builder.text(text);
  }

  page
    .execute(builder.build().map_err(|error| error.to_string())?)
    .await
    .map_err(|error| error.to_string())?;

  page
    .execute(
      DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .key(definition.key)
        .code(definition.code)
        .windows_virtual_key_code(definition.key_code)
        .native_virtual_key_code(definition.key_code)
        .build()
        .map_err(|error| error.to_string())?,
    )
    .await
    .map_err(|error| error.to_string())?;

  Ok(())
}
