use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillFormElement {
  #[schemars(description = "The uid of the element to fill out")]
  pub uid: String,
  #[schemars(
    description = "Value for the element. \"true\" or \"false\" for checkboxes and toggles, \"true\" for radio buttons."
  )]
  pub value: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FillFormRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(description = "Elements from snapshot to fill out.")]
  pub elements: Vec<FillFormElement>,
  #[schemars(description = "Whether to include a snapshot in the response. Default is false.")]
  pub include_snapshot: Option<bool>,
}

pub async fn fill_form(
  Parameters(FillFormRequest {
    page_id,
    elements,
    include_snapshot,
  }): Parameters<FillFormRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, include_snapshot);
  for element in elements {
    let _ = (element.uid, element.value);
  }
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
