use std::path::PathBuf;

use rmcp::ServiceExt;

use crate::browser::BrowserInstance;
use crate::browser::chrome_binary_default;
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
}

pub async fn main(args: StdioCommand) -> anyhow::Result<()> {
  let chrome_binary_path = match args.chrome_binary_path {
    Some(path) => path,
    None => chrome_binary_default()?,
  };

  if !chrome_binary_path.is_file() {
    return Err(anyhow::anyhow!(
      "chrome binary not found at {}",
      chrome_binary_path.display()
    ));
  }

  let browser = BrowserInstance::launch(&chrome_binary_path, args.headless).await?;
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
        eprintln!("Update available: {} -> {}  (run `browser-mcp update`)", version, latest);
      }
    }
  });

  let server = BrowserServer::new(tool_context)
    .serve(rmcp::transport::stdio())
    .await?;

  server.waiting().await?;
  Ok(())
}
