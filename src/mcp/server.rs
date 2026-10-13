use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CacheScope;
use rmcp::model::CallToolResult;
use rmcp::model::ErrorData;
use rmcp::model::ListToolsResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ProtocolVersion;
use rmcp::model::ResultType;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerConfig;
use rmcp::service::RequestContext;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;

use crate::mcp::schema;
use crate::mcp::tool_context::ToolContext;
use crate::mcp::tools::click::ClickRequest;
use crate::mcp::tools::click::click;
use crate::mcp::tools::close_page::ClosePageRequest;
use crate::mcp::tools::close_page::close_page;
use crate::mcp::tools::drag::DragRequest;
use crate::mcp::tools::drag::drag;
use crate::mcp::tools::emulate::EmulateRequest;
use crate::mcp::tools::emulate::emulate;
use crate::mcp::tools::evaluate_script::EvaluateScriptRequest;
use crate::mcp::tools::evaluate_script::evaluate_script;
use crate::mcp::tools::fill::FillRequest;
use crate::mcp::tools::fill::fill;
use crate::mcp::tools::fill_form::FillFormRequest;
use crate::mcp::tools::fill_form::fill_form;
use crate::mcp::tools::get_console_message::GetConsoleMessageRequest;
use crate::mcp::tools::get_console_message::get_console_message;
use crate::mcp::tools::get_css_styles::GetCssStylesRequest;
use crate::mcp::tools::get_css_styles::get_css_styles;
use crate::mcp::tools::get_network_request::GetNetworkRequestRequest;
use crate::mcp::tools::get_network_request::get_network_request;
use crate::mcp::tools::handle_dialog::HandleDialogRequest;
use crate::mcp::tools::handle_dialog::handle_dialog;
use crate::mcp::tools::hover::HoverRequest;
use crate::mcp::tools::hover::hover;
use crate::mcp::tools::lighthouse_audit::LighthouseAuditRequest;
use crate::mcp::tools::lighthouse_audit::lighthouse_audit;
use crate::mcp::tools::list_console_messages::ListConsoleMessagesRequest;
use crate::mcp::tools::list_console_messages::list_console_messages;
use crate::mcp::tools::list_network_requests::ListNetworkRequestsRequest;
use crate::mcp::tools::list_network_requests::list_network_requests;
use crate::mcp::tools::list_pages::ListPagesRequest;
use crate::mcp::tools::list_pages::list_pages;
use crate::mcp::tools::navigate_page::NavigatePageRequest;
use crate::mcp::tools::navigate_page::navigate_page;
use crate::mcp::tools::new_page::NewPageRequest;
use crate::mcp::tools::new_page::new_page;
use crate::mcp::tools::performance_analyze_insight::PerformanceAnalyzeInsightRequest;
use crate::mcp::tools::performance_analyze_insight::performance_analyze_insight;
use crate::mcp::tools::performance_start_trace::PerformanceStartTraceRequest;
use crate::mcp::tools::performance_start_trace::performance_start_trace;
use crate::mcp::tools::performance_stop_trace::PerformanceStopTraceRequest;
use crate::mcp::tools::performance_stop_trace::performance_stop_trace;
use crate::mcp::tools::press_key::PressKeyRequest;
use crate::mcp::tools::press_key::press_key;
use crate::mcp::tools::resize_page::ResizePageRequest;
use crate::mcp::tools::resize_page::resize_page;
use crate::mcp::tools::select_page::SelectPageRequest;
use crate::mcp::tools::select_page::select_page;
use crate::mcp::tools::take_heapsnapshot::TakeHeapsnapshotRequest;
use crate::mcp::tools::take_heapsnapshot::take_heapsnapshot;
use crate::mcp::tools::take_screenshot::TakeScreenshotRequest;
use crate::mcp::tools::take_screenshot::take_screenshot;
use crate::mcp::tools::take_snapshot::TakeSnapshotRequest;
use crate::mcp::tools::take_snapshot::take_snapshot;
use crate::mcp::tools::type_text::TypeTextRequest;
use crate::mcp::tools::type_text::type_text;
use crate::mcp::tools::upload_file::UploadFileRequest;
use crate::mcp::tools::upload_file::upload_file;
use crate::mcp::tools::wait_for::WaitForRequest;
use crate::mcp::tools::wait_for::wait_for;

#[derive(Debug, Clone)]
pub struct BrowserServer {
  tool_router: ToolRouter<Self>,
  tool_context: ToolContext,
}

impl BrowserServer {
  pub fn new(tool_context: ToolContext) -> Self {
    Self {
      tool_router: Self::tool_router(),
      tool_context,
    }
  }
}

impl Default for BrowserServer {
  fn default() -> Self {
    Self::new(ToolContext::default())
  }
}

#[tool_router]
impl BrowserServer {
  #[tool(description = "Clicks on the provided element")]
  async fn click(
    &self,
    Parameters(request): Parameters<ClickRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    click(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Closes the page by its index. The last open page cannot be closed.")]
  async fn close_page(
    &self,
    Parameters(request): Parameters<ClosePageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    close_page(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Drag an element onto another element")]
  async fn drag(
    &self,
    Parameters(request): Parameters<DragRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    drag(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Emulates various features on the target page.")]
  async fn emulate(
    &self,
    Parameters(request): Parameters<EmulateRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    emulate(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Evaluate a JavaScript function inside the target page. Returns the response as JSON, so returned values have to be JSON-serializable."
  )]
  async fn evaluate_script(
    &self,
    Parameters(request): Parameters<EvaluateScriptRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    evaluate_script(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Type text into an input, text area or select an option from a <select> element."
  )]
  async fn fill(
    &self,
    Parameters(request): Parameters<FillRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    fill(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Fill out multiple form elements (inputs, selects, checkboxes, radios) at once. ALWAYS prefer this tool over multiple individual 'fill' or 'click' calls when interacting with forms. It is significantly faster, more reliable, and reduces turn count. Example: Fill username, password, and check \"Remember Me\" in one call."
  )]
  async fn fill_form(
    &self,
    Parameters(request): Parameters<FillFormRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    fill_form(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Gets a console message by its ID. You can get all messages by calling list_console_messages."
  )]
  async fn get_console_message(
    &self,
    Parameters(request): Parameters<GetConsoleMessageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    get_console_message(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Retrieve matched CSS rules, inline styles, inherited styles, and cascade information for an element identified by its UID.\nUse this tool to debug why specific CSS properties are applied, overridden, or conflicting. Results are paginated and return 10 rules per page by default; use pageIdx to page through the remaining rules. Requires a UID from take_snapshot."
  )]
  async fn get_css_styles(
    &self,
    Parameters(request): Parameters<GetCssStylesRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    get_css_styles(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Gets a network request by an optional reqid, if omitted returns the currently selected request in the DevTools Network panel. Useful for inspecting request headers (including 'Cookie') and response headers (including 'Set-Cookie' and directives)."
  )]
  async fn get_network_request(
    &self,
    Parameters(request): Parameters<GetNetworkRequestRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    get_network_request(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "If a browser dialog was opened, use this command to handle it")]
  async fn handle_dialog(
    &self,
    Parameters(request): Parameters<HandleDialogRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    handle_dialog(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Hover over the provided element")]
  async fn hover(
    &self,
    Parameters(request): Parameters<HoverRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    hover(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Get Lighthouse score and reports for accessibility, SEO, best practices, and agentic browsing. This excludes performance. For performance audits, run performance_start_trace"
  )]
  async fn lighthouse_audit(
    &self,
    Parameters(request): Parameters<LighthouseAuditRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    lighthouse_audit(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "List all console messages for the target page since the last navigation.")]
  async fn list_console_messages(
    &self,
    Parameters(request): Parameters<ListConsoleMessagesRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    list_console_messages(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Lists the most recent requests for the target page since the last navigation."
  )]
  async fn list_network_requests(
    &self,
    Parameters(request): Parameters<ListNetworkRequestsRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    list_network_requests(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Get a list of pages open in the browser.")]
  async fn list_pages(
    &self,
    Parameters(request): Parameters<ListPagesRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    list_pages(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Go to a URL, or back, forward, or reload. Use project URL if not specified otherwise."
  )]
  async fn navigate_page(
    &self,
    Parameters(request): Parameters<NavigatePageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    navigate_page(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Open a new tab and load a URL. Use project URL if not specified otherwise."
  )]
  async fn new_page(
    &self,
    Parameters(request): Parameters<NewPageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    new_page(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Provides more detailed information on a specific Performance Insight of an insight set that was highlighted in the results of a trace recording."
  )]
  async fn performance_analyze_insight(
    &self,
    Parameters(request): Parameters<PerformanceAnalyzeInsightRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    performance_analyze_insight(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Start a performance trace on the target webpage. Use to find frontend performance issues, Core Web Vitals (LCP, INP, CLS), and improve page load speed."
  )]
  async fn performance_start_trace(
    &self,
    Parameters(request): Parameters<PerformanceStartTraceRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    performance_start_trace(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Stop the active performance trace recording on the target webpage.")]
  async fn performance_stop_trace(
    &self,
    Parameters(request): Parameters<PerformanceStopTraceRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    performance_stop_trace(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Press a key or key combination. Use this when other input methods like fill() cannot be used (e.g., keyboard shortcuts, navigation keys, or special key combinations)."
  )]
  async fn press_key(
    &self,
    Parameters(request): Parameters<PressKeyRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    press_key(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Resizes the page's window so that the page has specified dimension")]
  async fn resize_page(
    &self,
    Parameters(request): Parameters<ResizePageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    resize_page(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Select a page as a context for future tool calls.")]
  async fn select_page(
    &self,
    Parameters(request): Parameters<SelectPageRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    select_page(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Capture a heap snapshot of the target page. Use to analyze the memory distribution of JavaScript objects and debug memory leaks."
  )]
  async fn take_heapsnapshot(
    &self,
    Parameters(request): Parameters<TakeHeapsnapshotRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    take_heapsnapshot(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Take a screenshot of the page or element.")]
  async fn take_screenshot(
    &self,
    Parameters(request): Parameters<TakeScreenshotRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    take_screenshot(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(
    description = "Take a text snapshot of the target page based on the a11y tree. The snapshot lists page elements along with a unique\nidentifier (uid). Always use the latest snapshot. Prefer taking a snapshot over taking a screenshot. The snapshot indicates the element selected\nin the DevTools Elements panel (if any)."
  )]
  async fn take_snapshot(
    &self,
    Parameters(request): Parameters<TakeSnapshotRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    take_snapshot(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Type text using keyboard into a previously focused input")]
  async fn type_text(
    &self,
    Parameters(request): Parameters<TypeTextRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    type_text(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Upload a file through a provided element.")]
  async fn upload_file(
    &self,
    Parameters(request): Parameters<UploadFileRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    upload_file(self.tool_context.clone(), Parameters(request)).await
  }

  #[tool(description = "Wait for the specified text to appear on the selected page.")]
  async fn wait_for(
    &self,
    Parameters(request): Parameters<WaitForRequest>,
  ) -> Result<CallToolResult, ErrorData> {
    wait_for(self.tool_context.clone(), Parameters(request)).await
  }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BrowserServer {
  fn get_info(&self) -> ServerConfig {
    ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
      .with_instructions("Controls a browser over the Chrome DevTools Protocol")
  }

  async fn list_tools(
    &self,
    _request: Option<PaginatedRequestParams>,
    context: RequestContext<RoleServer>,
  ) -> Result<ListToolsResult, ErrorData> {
    let supports_cache_hints = context
      .protocol_version()
      .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28);

    let tools = self
      .tool_router
      .list_all()
      .into_iter()
      .map(|mut tool| {
        let mut schema = (*tool.input_schema).clone();
        schema::normalize(&tool.name, &mut schema);
        tool.input_schema = std::sync::Arc::new(schema);
        tool
      })
      .collect();

    Ok(ListToolsResult {
      result_type: Some(ResultType::COMPLETE),
      tools,
      meta: None,
      next_cursor: None,
      ttl_ms: supports_cache_hints.then_some(0),
      cache_scope: supports_cache_hints.then_some(CacheScope::Public),
    })
  }
}
