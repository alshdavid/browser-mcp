use rmcp::ServiceExt;

use crate::mcp::server::BrowserServer;

#[derive(Debug, clap::Parser)]
pub struct StdioCommand {}

pub async fn main(_args: StdioCommand) -> anyhow::Result<()> {
  let server = BrowserServer::new().serve(rmcp::transport::stdio()).await?;
  server.waiting().await?;
  Ok(())
}
