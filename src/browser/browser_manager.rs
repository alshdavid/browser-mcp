use std::path::PathBuf;

use tokio::sync::Mutex;

use crate::browser::BrowserInstance;
use crate::browser::chrome_binary_default;
use crate::browser::chrome_user_data_dir;
use crate::browser::devtools_active_port_ws_url;

#[derive(Debug, Clone)]
pub struct BrowserOptions {
  pub chrome_binary_path: Option<PathBuf>,
  pub headless: bool,
  pub auto_connect: bool,
}

#[derive(Debug, Clone)]
pub struct BrowserManager {
  options: BrowserOptions,
  instance: std::sync::Arc<Mutex<Option<BrowserInstance>>>,
}

impl BrowserManager {
  pub fn new(options: BrowserOptions) -> Self {
    Self {
      options,
      instance: std::sync::Arc::new(Mutex::new(None)),
    }
  }

  pub async fn ensure(&self) -> anyhow::Result<BrowserInstance> {
    let mut instance = self.instance.lock().await;
    if instance.is_none() {
      *instance = Some(connect_or_launch(&self.options).await?);
    }

    Ok(instance.as_ref().expect("browser was just connected").clone())
  }
}

async fn connect_or_launch(options: &BrowserOptions) -> anyhow::Result<BrowserInstance> {
  if options.auto_connect {
    connect_to_running_chrome().await
  } else {
    launch_chrome(options).await
  }
}

async fn launch_chrome(options: &BrowserOptions) -> anyhow::Result<BrowserInstance> {
  let chrome_binary_path = match options.chrome_binary_path.as_ref() {
    Some(path) => {
      if !path.is_file() {
        return Err(anyhow::anyhow!(
          "chrome binary not found at {}",
          path.display()
        ));
      }
      path.clone()
    }
    None => chrome_binary_default()?,
  };

  BrowserInstance::launch(&chrome_binary_path, options.headless).await
}

async fn connect_to_running_chrome() -> anyhow::Result<BrowserInstance> {
  let user_data_dir = chrome_user_data_dir()?;
  let web_socket_url = devtools_active_port_ws_url(&user_data_dir)?;

  BrowserInstance::connect(&web_socket_url).await
}