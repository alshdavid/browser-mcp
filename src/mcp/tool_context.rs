use std::collections::HashMap;
use std::sync::Arc;

use chromiumoxide::Browser;
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use tokio::sync::Mutex;

use crate::browser::BrowserInstance;

#[derive(Debug, Clone)]
pub struct ToolContext {
  browser: Arc<Mutex<BrowserInstance>>,
  isolated_contexts: Arc<Mutex<HashMap<String, BrowserContextId>>>,
}

impl ToolContext {
  pub fn new(browser: BrowserInstance) -> Self {
    Self {
      browser: Arc::new(Mutex::new(browser)),
      isolated_contexts: Arc::new(Mutex::new(HashMap::new())),
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
}
