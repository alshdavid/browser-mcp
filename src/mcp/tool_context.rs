use std::collections::HashMap;
use std::sync::Arc;

use chromiumoxide::Browser;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::page::EventJavascriptDialogOpening;
use chromiumoxide::cdp::browser_protocol::target::TargetId;
use futures_util::StreamExt as _;
use tokio::sync::Mutex;

use crate::browser::BrowserInstance;

#[derive(Debug, Clone)]
struct PageEntry {
  id: f64,
  page: Page,
  snapshot: Option<PageSnapshot>,
  dialog_open: bool,
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

#[derive(Debug, Clone)]
pub struct ToolContext {
  browser: Arc<Mutex<BrowserInstance>>,
  isolated_contexts: Arc<Mutex<HashMap<String, BrowserContextId>>>,
  pages: Arc<Mutex<Vec<PageEntry>>>,
  next_page_id: Arc<Mutex<f64>>,
  next_snapshot_id: Arc<Mutex<u64>>,
}

impl ToolContext {
  pub fn new(browser: BrowserInstance) -> Self {
    Self {
      browser: Arc::new(Mutex::new(browser)),
      isolated_contexts: Arc::new(Mutex::new(HashMap::new())),
      pages: Arc::new(Mutex::new(Vec::new())),
      next_page_id: Arc::new(Mutex::new(1.0)),
      next_snapshot_id: Arc::new(Mutex::new(1)),
    }
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
    });

    watch_dialogs(page, id, self.pages.clone());

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
