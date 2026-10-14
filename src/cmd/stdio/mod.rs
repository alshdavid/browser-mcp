use std::path::PathBuf;

use rmcp::ServiceExt;

use crate::browser::BrowserManager;
use crate::browser::BrowserOptions;
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
  let tool_context = ToolContext::new(BrowserManager::new(BrowserOptions {
    chrome_binary_path: args.chrome_binary_path,
    headless: args.headless,
    auto_connect: args.auto_connect,
  }));

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
