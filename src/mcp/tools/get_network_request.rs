use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetNetworkRequestRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The reqid of the network request. If omitted returns the currently selected request in the DevTools Network panel."
  )]
  pub reqid: Option<f64>,
  #[schemars(
    description = "The absolute or relative path to a .network-request file to save the request body to. If omitted, the body is returned inline."
  )]
  pub request_file_path: Option<String>,
  #[schemars(
    description = "The absolute or relative path to a .network-response file to save the response body to. If omitted, the body is returned inline."
  )]
  pub response_file_path: Option<String>,
}

pub async fn get_network_request(
  Parameters(GetNetworkRequestRequest {
    page_id,
    reqid,
    request_file_path,
    response_file_path,
  }): Parameters<GetNetworkRequestRequest>
) -> Result<CallToolResult, ErrorData> {
  let _ = (page_id, reqid, request_file_path, response_file_path);
  Ok(CallToolResult::success(vec![ContentBlock::text(
    "not implemented",
  )]))
}
