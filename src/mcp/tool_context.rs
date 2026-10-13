use std::collections::HashMap;
use std::sync::Arc;

use chromiumoxide::Browser;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::network::EnableParams;
use chromiumoxide::cdp::browser_protocol::network::EventLoadingFailed;
use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::browser_protocol::network::Headers;
use chromiumoxide::cdp::browser_protocol::network::ResourceType;
use chromiumoxide::cdp::browser_protocol::page::EventFrameNavigated;
use chromiumoxide::cdp::browser_protocol::page::EventJavascriptDialogOpening;
use chromiumoxide::cdp::browser_protocol::target::TargetId;
use chromiumoxide::cdp::js_protocol::runtime::CallFrame;
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::cdp::js_protocol::runtime::EventExceptionThrown;
use chromiumoxide::cdp::js_protocol::runtime::RemoteObject;
use chromiumoxide::cdp::js_protocol::runtime::RemoteObjectType;
use chromiumoxide::cdp::js_protocol::runtime::StackTrace;
use futures_util::StreamExt as _;
use tokio::sync::Mutex;

use crate::browser::BrowserInstance;

#[derive(Debug, Clone)]
struct PageEntry {
  id: f64,
  page: Page,
  snapshot: Option<PageSnapshot>,
  dialog_open: bool,
  console: ConsoleStorage,
  network: NetworkStorage,
}

#[derive(Debug, Clone, Default)]
pub struct PageSnapshot {
  pub uids_by_backend_node: HashMap<i64, String>,
  pub nodes_by_uid: HashMap<String, SnapshotNode>,
}

#[derive(Debug, Clone)]
pub struct SnapshotNode {
  pub backend_node_id: Option<i64>,
  pub role: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConsoleMessage {
  pub id: u64,
  pub message_type: String,
  pub text: String,
  pub args: Vec<String>,
  pub frames: Vec<ConsoleFrame>,
  pub preserved: bool,
}

#[derive(Debug, Clone)]
pub struct ConsoleFrame {
  pub function_name: String,
  pub url: String,
  pub line_number: i64,
  pub column_number: i64,
}

#[derive(Debug, Clone, Default)]
struct ConsoleStorage {
  // Index 0 is the navigation in progress; deeper entries are the preserved
  // navigations, newest first, capped so only the last three are retained.
  navigations: Vec<Vec<ConsoleMessage>>,
  next_id: u64,
}

impl ConsoleStorage {
  fn collect(
    &mut self,
    mut message: ConsoleMessage,
  ) {
    if self.navigations.is_empty() {
      self.navigations.push(Vec::new());
    }
    self.next_id += 1;
    message.id = self.next_id;
    self.navigations[0].push(message);
  }

  fn split(&mut self) {
    self.navigations.insert(0, Vec::new());
    self.navigations.truncate(4);
  }

  fn find(
    &self,
    id: u64,
  ) -> Option<ConsoleMessage> {
    self
      .navigations
      .iter()
      .enumerate()
      .find_map(|(index, navigation)| {
        navigation
          .iter()
          .find(|message| message.id == id)
          .cloned()
          .map(|mut message| {
            message.preserved = index > 0;
            message
          })
      })
  }

  fn messages(
    &self,
    include_preserved: bool,
  ) -> Vec<ConsoleMessage> {
    if !include_preserved {
      return self.navigations.first().cloned().unwrap_or_default();
    }

    let mut messages = Vec::new();
    for (index, navigation) in self.navigations.iter().enumerate().rev() {
      messages.extend(navigation.iter().cloned().map(|mut message| {
        message.preserved = index > 0;
        message
      }));
    }
    messages
  }
}

#[derive(Debug, Clone, Default)]
pub struct NetworkRequest {
  pub id: u64,
  pub cdp_request_id: String,
  pub method: String,
  pub url: String,
  pub status: Option<i64>,
  pub status_text: Option<String>,
  pub error_text: Option<String>,
  pub resource_type: Option<String>,
  pub request_headers: Vec<(String, String)>,
  pub response_headers: Vec<(String, String)>,
  pub is_navigation_request: bool,
  pub has_post_data: bool,
  pub mime_type: Option<String>,
  pub preserved: bool,
  pub redirect_chain: Vec<RedirectHop>,
}

#[derive(Debug, Clone, Default)]
pub struct RedirectHop {
  pub reqid: u64,
  pub method: String,
  pub url: String,
  pub status: String,
}

#[derive(Debug, Clone, Default)]
struct NetworkStorage {
  navigations: Vec<Vec<NetworkRequest>>,
  next_id: u64,
}

impl NetworkStorage {
  fn collect(
    &mut self,
    mut request: NetworkRequest,
  ) {
    if self.navigations.is_empty() {
      self.navigations.push(Vec::new());
    }
    self.next_id += 1;
    request.id = self.next_id;
    let navigation = &mut self.navigations[0];
    navigation.push(request);
    if navigation.len() > 1_000 {
      let excess = navigation.len() - 1_000;
      navigation.drain(..excess);
    }
  }

  // Upstream overrides the console split rule: the request that starts the new
  // navigation is emitted before the frameNavigated event that ends the old
  // one, so the trailing run from the last main-frame navigation request moves
  // into the new bucket together with that request itself.
  fn split(&mut self) {
    let split_at = self.navigations.first().and_then(|navigation| {
      navigation
        .iter()
        .rposition(|request| request.is_navigation_request)
    });

    let carried = match split_at {
      Some(index) => self.navigations[0].split_off(index),
      None => Vec::new(),
    };

    self.navigations.insert(0, carried);
    self.navigations.truncate(4);
  }

  fn find(
    &self,
    id: u64,
  ) -> Option<NetworkRequest> {
    self
      .navigations
      .iter()
      .enumerate()
      .find_map(|(index, navigation)| {
        navigation
          .iter()
          .find(|request| request.id == id)
          .cloned()
          .map(|mut request| {
            request.preserved = index > 0;
            request
          })
      })
  }

  fn requests(
    &self,
    include_preserved: bool,
  ) -> Vec<NetworkRequest> {
    if !include_preserved {
      return self.navigations.first().cloned().unwrap_or_default();
    }

    let mut requests = Vec::new();
    for (index, navigation) in self.navigations.iter().enumerate().rev() {
      requests.extend(navigation.iter().cloned().map(|mut request| {
        request.preserved = index > 0;
        request
      }));
    }
    requests
  }
}

#[derive(Debug, Clone)]
pub struct RecordedTrace {
  pub url: String,
  pub raw: Vec<u8>,
  pub events: Vec<serde_json::Value>,
  pub event_count: usize,
  pub duration_seconds: f64,
  pub insights: Vec<TraceInsightSet>,
}

#[derive(Debug, Clone, Default)]
pub struct TraceInsightSet {
  pub insight_set_id: String,
  pub url: String,
  pub insights: HashMap<String, TraceInsight>,
}

#[derive(Debug, Clone, Default)]
pub struct TraceInsight {
  pub name: String,
  pub title: String,
  pub description: String,
  pub metrics: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
  browser: Arc<Mutex<BrowserInstance>>,
  isolated_contexts: Arc<Mutex<HashMap<String, BrowserContextId>>>,
  pages: Arc<Mutex<Vec<PageEntry>>>,
  next_page_id: Arc<Mutex<f64>>,
  next_snapshot_id: Arc<Mutex<u64>>,
  running_performance_trace: Arc<Mutex<bool>>,
  recorded_traces: Arc<Mutex<Vec<RecordedTrace>>>,
}

impl ToolContext {
  pub fn new(browser: BrowserInstance) -> Self {
    Self {
      browser: Arc::new(Mutex::new(browser)),
      isolated_contexts: Arc::new(Mutex::new(HashMap::new())),
      pages: Arc::new(Mutex::new(Vec::new())),
      next_page_id: Arc::new(Mutex::new(1.0)),
      next_snapshot_id: Arc::new(Mutex::new(1)),
      running_performance_trace: Arc::new(Mutex::new(false)),
      recorded_traces: Arc::new(Mutex::new(Vec::new())),
    }
  }

  pub async fn is_running_performance_trace(&self) -> bool {
    *self.running_performance_trace.lock().await
  }

  pub async fn set_running_performance_trace(
    &self,
    running: bool,
  ) {
    *self.running_performance_trace.lock().await = running;
  }

  pub async fn store_trace_recording(
    &self,
    trace: RecordedTrace,
  ) {
    let mut traces = self.recorded_traces.lock().await;
    traces.push(trace);
    // Only the most recent recording is reachable, so retaining more would leak
    // a full raw trace per run for the life of the session.
    if traces.len() > 1 {
      let excess = traces.len() - 1;
      traces.drain(..excess);
    }
  }

  pub async fn last_recorded_trace(&self) -> Option<RecordedTrace> {
    self.recorded_traces.lock().await.last().cloned()
  }

  pub async fn with_browser<T>(
    &self,
    f: impl AsyncFnOnce(&Browser) -> T,
  ) -> T {
    let browser = self.browser.lock().await;
    f(browser.browser()).await
  }

  pub async fn isolated_context(
    &self,
    name: &str,
  ) -> Option<BrowserContextId> {
    self.isolated_contexts.lock().await.get(name).cloned()
  }

  pub async fn set_isolated_context(
    &self,
    name: String,
    id: BrowserContextId,
  ) {
    self.isolated_contexts.lock().await.insert(name, id);
  }

  pub async fn register_page(
    &self,
    page: Page,
  ) -> f64 {
    let mut next_page_id = self.next_page_id.lock().await;
    let id = *next_page_id;
    *next_page_id += 1.0;
    drop(next_page_id);

    self.pages.lock().await.push(PageEntry {
      id,
      page: page.clone(),
      snapshot: None,
      dialog_open: false,
      console: ConsoleStorage::default(),
      network: NetworkStorage::default(),
    });

    watch_dialogs(page.clone(), id, self.pages.clone());
    collect_console_messages(page.clone(), id, self.pages.clone());
    collect_network_requests(page, id, self.pages.clone());

    id
  }

  pub async fn page(
    &self,
    id: f64,
  ) -> Option<Page> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == id)
      .map(|entry| entry.page.clone())
  }

  pub async fn page_count(&self) -> usize {
    self.pages.lock().await.len()
  }

  pub async fn drop_page_by_target(
    &self,
    target_id: &TargetId,
  ) {
    self
      .pages
      .lock()
      .await
      .retain(|entry| entry.page.target_id() != target_id);
  }

  pub async fn console_messages(
    &self,
    page_id: f64,
    include_preserved: bool,
  ) -> Vec<ConsoleMessage> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .map(|entry| entry.console.messages(include_preserved))
      .unwrap_or_default()
  }

  pub async fn console_message(
    &self,
    page_id: f64,
    msgid: u64,
  ) -> Option<ConsoleMessage> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.console.find(msgid))
  }

  pub async fn network_requests(
    &self,
    page_id: f64,
    include_preserved: bool,
  ) -> Vec<NetworkRequest> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .map(|entry| entry.network.requests(include_preserved))
      .unwrap_or_default()
  }

  pub async fn network_request(
    &self,
    page_id: f64,
    reqid: u64,
  ) -> Option<NetworkRequest> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.network.find(reqid))
  }

  pub async fn dialog_open(
    &self,
    page_id: f64,
  ) -> bool {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .is_some_and(|entry| entry.dialog_open)
  }

  pub async fn clear_dialog(
    &self,
    page_id: f64,
  ) {
    let mut pages = self.pages.lock().await;
    if let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) {
      entry.dialog_open = false;
    }
  }

  pub async fn begin_snapshot(
    &self,
    page_id: f64,
  ) -> (u64, HashMap<i64, String>) {
    let mut next_snapshot_id = self.next_snapshot_id.lock().await;
    let snapshot_id = *next_snapshot_id;
    *next_snapshot_id += 1;
    drop(next_snapshot_id);

    let previous = self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.snapshot.as_ref())
      .map(|snapshot| snapshot.uids_by_backend_node.clone())
      .unwrap_or_default();

    (snapshot_id, previous)
  }

  pub async fn backend_node_for_uid(
    &self,
    page_id: f64,
    uid: &str,
  ) -> Option<i64> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.snapshot.as_ref())
      .and_then(|snapshot| {
        snapshot
          .uids_by_backend_node
          .iter()
          .find(|(_, value)| value.as_str() == uid)
          .map(|(backend_node_id, _)| *backend_node_id)
      })
  }

  pub async fn uids_by_backend_node(
    &self,
    page_id: f64,
  ) -> HashMap<i64, String> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.snapshot.as_ref())
      .map(|snapshot| snapshot.uids_by_backend_node.clone())
      .unwrap_or_default()
  }

  pub async fn node_for_uid(
    &self,
    page_id: f64,
    uid: &str,
  ) -> Option<SnapshotNode> {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .and_then(|entry| entry.snapshot.as_ref())
      .and_then(|snapshot| snapshot.nodes_by_uid.get(uid).cloned())
  }

  pub async fn has_snapshot(
    &self,
    page_id: f64,
  ) -> bool {
    self
      .pages
      .lock()
      .await
      .iter()
      .find(|entry| entry.id == page_id)
      .is_some_and(|entry| entry.snapshot.is_some())
  }

  pub async fn store_snapshot(
    &self,
    page_id: f64,
    snapshot: PageSnapshot,
  ) {
    let mut pages = self.pages.lock().await;
    if let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) {
      entry.snapshot = Some(snapshot);
    }
  }
}

fn watch_dialogs(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  // The dialog-opening event fires during whatever tool call triggers it, so the
  // subscription must be live from page registration rather than from
  // handle_dialog, which would already be too late to observe it.
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventJavascriptDialogOpening>().await else {
      return;
    };

    while events.next().await.is_some() {
      let mut pages = pages.lock().await;
      if let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) {
        entry.dialog_open = true;
      }
    }
  });
}

fn collect_console_messages(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  // Console events only carry data while a listener is attached, so the
  // subscriptions have to exist from page registration rather than from the
  // list_console_messages call that reads them.
  watch_console_api_calls(page.clone(), page_id, pages.clone());
  watch_exceptions(page.clone(), page_id, pages.clone());
  watch_navigations(page, page_id, pages);
}

fn watch_console_api_calls(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventConsoleApiCalled>().await else {
      return;
    };

    while let Some(event) = events.next().await {
      let args = event
        .args
        .iter()
        .map(format_remote_object)
        .collect::<Vec<_>>();

      let message = ConsoleMessage {
        message_type: normalize_type(event.r#type.as_ref()),
        text: args.join(" "),
        args,
        frames: stack_frames(event.stack_trace.as_ref()),
        ..ConsoleMessage::default()
      };

      push_console_message(&pages, page_id, message).await;
    }
  });
}

fn watch_exceptions(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventExceptionThrown>().await else {
      return;
    };

    while let Some(event) = events.next().await {
      let details = &event.exception_details;
      let exception = details.exception.as_ref().map(format_remote_object);
      let location = exception_location(details);

      let text = match (exception, details.text.as_str()) {
        (Some(exception), "Uncaught") => match location {
          Some(location) => format!("Uncaught {exception}\n    at {location}"),
          None => format!("Uncaught {exception}"),
        },
        (_, text) => text.to_owned(),
      };

      let message = ConsoleMessage {
        message_type: "error".to_owned(),
        text,
        args: Vec::new(),
        frames: stack_frames(details.stack_trace.as_ref()),
        ..ConsoleMessage::default()
      };

      push_console_message(&pages, page_id, message).await;
    }
  });
}

fn exception_location(
  details: &chromiumoxide::cdp::js_protocol::runtime::ExceptionDetails
) -> Option<String> {
  let url = details.url.clone().unwrap_or_default();
  let line = details.line_number + 1;
  let column = details.column_number + 1;

  if url.is_empty() && line <= 1 && column <= 1 {
    return None;
  }

  Some(format!("{url}:{line}:{column}"))
}

fn normalize_type(r#type: &str) -> String {
  match r#type {
    "warning" => "warn".to_owned(),
    other => other.to_owned(),
  }
}

fn watch_navigations(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventFrameNavigated>().await else {
      return;
    };

    while let Some(event) = events.next().await {
      // Only main-frame navigations rotate the preserved history; sub-frame
      // navigations would otherwise discard the messages still on screen.
      if event.frame.parent_id.is_some() {
        continue;
      }

      let mut pages = pages.lock().await;
      if let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) {
        entry.console.split();
        entry.network.split();
      }
    }
  });
}

fn collect_network_requests(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  // Network events are only delivered while Network.enable is in effect and a
  // listener is attached, so both are established at page registration rather
  // than from the list_network_requests call that reads the storage.
  watch_network_responses(page.clone(), page_id, pages.clone());
  watch_network_failures(page.clone(), page_id, pages.clone());

  tokio::spawn(async move {
    let _ = page.execute(EnableParams::default()).await;

    let Ok(mut events) = page.event_listener::<EventRequestWillBeSent>().await else {
      return;
    };

    // Resolved per event: a cross-process navigation can replace the frame, and
    // a one-shot lookup at registration time may briefly return nothing.
    let mut main_frame = None;

    while let Some(event) = events.next().await {
      let frame_id = event
        .frame_id
        .as_ref()
        .map(|frame| frame.as_ref().to_owned());
      let is_document = event.r#type == Some(ResourceType::Document);

      if is_document || main_frame.is_none() {
        main_frame = page.mainframe().await.ok().flatten();
      }

      let request = NetworkRequest {
        cdp_request_id: event.request_id.as_ref().to_owned(),
        method: event.request.method.clone(),
        url: event.request.url.clone(),
        status: None,
        resource_type: event.r#type.as_ref().map(|kind| kind.as_ref().to_owned()),
        request_headers: header_pairs(&event.request.headers),
        is_navigation_request: is_document
          && frame_id.as_deref() == main_frame.as_ref().map(|frame| frame.as_ref()),
        has_post_data: event.request.has_post_data.unwrap_or(false),
        ..NetworkRequest::default()
      };

      push_network_request(&pages, page_id, &event, request).await;
    }
  });
}

fn watch_network_responses(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventResponseReceived>().await else {
      return;
    };

    while let Some(event) = events.next().await {
      let response = &event.response;
      update_network_request(&pages, page_id, event.request_id.as_ref(), |request| {
        request.status = Some(response.status);
        request.status_text = Some(response.status_text.clone());
        request.response_headers = header_pairs(&response.headers);
        request.mime_type = Some(response.mime_type.clone());
        request.resource_type = Some(event.r#type.as_ref().to_owned());
        request.error_text = None;
      })
      .await;
    }
  });
}

fn watch_network_failures(
  page: Page,
  page_id: f64,
  pages: Arc<Mutex<Vec<PageEntry>>>,
) {
  tokio::spawn(async move {
    let Ok(mut events) = page.event_listener::<EventLoadingFailed>().await else {
      return;
    };

    while let Some(event) = events.next().await {
      update_network_request(&pages, page_id, event.request_id.as_ref(), |request| {
        request.error_text = Some(event.error_text.clone());
      })
      .await;
    }
  });
}

async fn push_network_request(
  pages: &Arc<Mutex<Vec<PageEntry>>>,
  page_id: f64,
  event: &EventRequestWillBeSent,
  request: NetworkRequest,
) {
  let mut pages = pages.lock().await;
  let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) else {
    return;
  };

  let previous = entry
    .network
    .navigations
    .iter_mut()
    .flat_map(|navigation| navigation.iter_mut())
    .rev()
    .find(|previous| previous.cdp_request_id == request.cdp_request_id);

  let mut request = request;
  request.redirect_chain = redirect_chain(event, previous);
  entry.network.collect(request);
}

async fn update_network_request(
  pages: &Arc<Mutex<Vec<PageEntry>>>,
  page_id: f64,
  cdp_request_id: &str,
  update: impl FnOnce(&mut NetworkRequest),
) {
  let mut pages = pages.lock().await;
  let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) else {
    return;
  };

  for navigation in entry.network.navigations.iter_mut() {
    if let Some(request) = navigation
      .iter_mut()
      .find(|request| request.cdp_request_id == cdp_request_id)
    {
      update(request);
      return;
    }
  }
}

// CDP reuses the same requestId along a redirect chain and reports each hop only
// as a `redirectResponse` on the event that replaces it, so the hop is recorded
// against the request already stored under that id. The member carries the chain
// accumulated so far, which the replacing request inherits.
fn redirect_chain(
  event: &EventRequestWillBeSent,
  previous: Option<&mut NetworkRequest>,
) -> Vec<RedirectHop> {
  let (Some(response), Some(previous)) = (event.redirect_response.as_ref(), previous) else {
    return Vec::new();
  };

  previous.status = Some(response.status);
  previous.status_text = Some(response.status_text.clone());
  previous.response_headers = header_pairs(&response.headers);
  previous.mime_type = Some(response.mime_type.clone());

  let mut chain = previous.redirect_chain.clone();
  chain.push(RedirectHop {
    reqid: previous.id,
    method: previous.method.clone(),
    url: previous.url.clone(),
    status: response.status.to_string(),
  });

  chain
}

fn header_pairs(headers: &Headers) -> Vec<(String, String)> {
  headers
    .inner()
    .as_object()
    .map(|headers| {
      headers
        .iter()
        .map(|(name, value)| (name.clone(), header_value(value)))
        .collect()
    })
    .unwrap_or_default()
}

fn header_value(value: &serde_json::Value) -> String {
  value
    .as_str()
    .map(str::to_owned)
    .unwrap_or_else(|| value.to_string())
}

async fn push_console_message(
  pages: &Arc<Mutex<Vec<PageEntry>>>,
  page_id: f64,
  message: ConsoleMessage,
) {
  let mut pages = pages.lock().await;
  if let Some(entry) = pages.iter_mut().find(|entry| entry.id == page_id) {
    entry.console.collect(message);
  }
}

fn stack_frames(stack_trace: Option<&StackTrace>) -> Vec<ConsoleFrame> {
  stack_trace
    .map(|trace| trace.call_frames.iter().map(console_frame).collect())
    .unwrap_or_default()
}

fn console_frame(frame: &CallFrame) -> ConsoleFrame {
  ConsoleFrame {
    function_name: frame.function_name.clone(),
    url: frame.url.clone(),
    line_number: frame.line_number,
    column_number: frame.column_number,
  }
}

fn format_remote_object(object: &RemoteObject) -> String {
  if let Some(value) = object.value.as_ref() {
    return match object.r#type {
      RemoteObjectType::String => value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string()),
      _ => value.to_string(),
    };
  }

  if let Some(value) = object.unserializable_value.as_ref() {
    return value.as_ref().to_owned();
  }

  object
    .description
    .clone()
    .unwrap_or_else(|| format!("JSHandle@{}", object.r#type.as_ref()))
}
