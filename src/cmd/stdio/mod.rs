use std::path::PathBuf;

use rmcp::ServiceExt;

use crate::browser::chrome_binary_default;
use crate::mcp::server::BrowserServer;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, clap::Parser)]
pub struct StdioCommand {
  #[arg(long = "chrome-binary-path", env = "CHROME_BINARY_PATH")]
  chrome_binary_path: Option<PathBuf>,
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

  let tool_context = ToolContext::default();

  let server = BrowserServer::new(tool_context)
    .serve(rmcp::transport::stdio())
    .await?;
  
  server.waiting().await?;
  Ok(())
}
