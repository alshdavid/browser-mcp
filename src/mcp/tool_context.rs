use std::collections::HashMap;
use std::sync::Arc;

use chromiumoxide::Browser;
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::target::TargetId;
use tokio::sync::Mutex;

use crate::browser::BrowserInstance;

#[derive(Debug, Clone)]
struct PageEntry {
  id: f64,
  page: Page,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
  browser: Arc<Mutex<BrowserInstance>>,
  isolated_contexts: Arc<Mutex<HashMap<String, BrowserContextId>>>,
  pages: Arc<Mutex<Vec<PageEntry>>>,
  next_page_id: Arc<Mutex<f64>>,
}

impl ToolContext {
  pub fn new(browser: BrowserInstance) -> Self {
    Self {
      browser: Arc::new(Mutex::new(browser)),
      isolated_contexts: Arc::new(Mutex::new(HashMap::new())),
      pages: Arc::new(Mutex::new(Vec::new())),
      next_page_id: Arc::new(Mutex::new(1.0)),
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

    self.pages.lock().await.push(PageEntry { id, page });

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
}
