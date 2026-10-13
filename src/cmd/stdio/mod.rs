use std::path::PathBuf;

use rmcp::ServiceExt;

use crate::browser::BrowserInstance;
use crate::browser::chrome_binary_default;
use crate::browser::chrome_user_data_dir;
use crate::browser::devtools_active_port_ws_url;
use crate::mcp::server::BrowserServer;
use crate::mcp::tool_context::ToolContext;
use crate::platform::self_upgrade;
use crate::platform::self_upgrade::TARGET_REPO;
use crate::platform::self_upgrade::UpgradeOptions;

#[derive(Debug, clap::Parser)]
pub struct StdioCommand {
  #[arg(long = "chrome-binary-path", env = "CHROME_BINARY_PATH")]
  chrome_binary_path: Option<PathBuf>,
  #[arg(long = "headless")]
  headless: bool,
  #[arg(long = "auto-connect", alias = "autoConnect")]
  auto_connect: bool,
}

pub async fn main(args: StdioCommand) -> anyhow::Result<()> {
  let browser = if args.auto_connect {
    connect_to_running_chrome().await?
  } else {
    launch_chrome(&args).await?
  };

  let tool_context = ToolContext::new(browser);

  tokio::spawn({
    let version = env!("CARGO_PKG_VERSION");
    async move {
      let available = self_upgrade::check_for_update(&UpgradeOptions {
        target_repo: TARGET_REPO,
        current_version: version,
      })
      .await;

      if let Ok(Some(latest)) = available {
        eprintln!(
          "Update available: {} -> {}  (run `browser-mcp update`)",
          version, latest
        );
      }
    }
  });

  let server = BrowserServer::new(tool_context)
    .serve(rmcp::transport::stdio())
    .await?;

  server.waiting().await?;
  Ok(())
}

async fn launch_chrome(args: &StdioCommand) -> anyhow::Result<BrowserInstance> {
  let chrome_binary_path = match args.chrome_binary_path.as_ref() {
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

  BrowserInstance::launch(&chrome_binary_path, args.headless).await
}

async fn connect_to_running_chrome() -> anyhow::Result<BrowserInstance> {
  let user_data_dir = chrome_user_data_dir()?;
  let web_socket_url = devtools_active_port_ws_url(&user_data_dir)?;

  BrowserInstance::connect(&web_socket_url).await
}
