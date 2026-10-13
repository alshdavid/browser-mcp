use std::path::Path;

use anyhow::Context as _;
use chromiumoxide::Browser;
use chromiumoxide::BrowserConfig;
use chromiumoxide::Handler;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct BrowserInstance {
  browser: Browser,
  handler: JoinHandle<()>,
}

impl BrowserInstance {
  pub async fn launch(
    chrome_binary_path: &Path,
    headless: bool,
  ) -> anyhow::Result<Self> {
    let mut config = BrowserConfig::builder()
      .chrome_executable(chrome_binary_path)
      .no_sandbox();

    if headless {
      config = config.new_headless_mode();
    } else {
      config = config.with_head();
    }

    let config = config
      .build()
      .map_err(anyhow::Error::msg)
      .context("failed to build the chrome launch config")?;

    let (browser, handler) = Browser::launch(config)
      .await
      .context("failed to launch chrome")?;

    Ok(Self {
      browser,
      handler: tokio::spawn(drive(handler)),
    })
  }

  pub fn browser(&self) -> &Browser {
    &self.browser
  }
}

async fn drive(mut handler: Handler) {
  use futures_util::StreamExt as _;

  while let Some(Ok(())) = handler.next().await {}
}
