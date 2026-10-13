mod browser;
mod cmd;
mod mcp;
mod platform;

use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Command {
  #[clap(subcommand)]
  command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
  #[clap(name = "stdio")]
  Stdio(cmd::stdio::StdioCommand),
  #[clap(name = "update")]
  Update(cmd::update::UpdateCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  match Command::parse().command {
    Commands::Stdio(args) => cmd::stdio::main(args).await,
    Commands::Update(args) => cmd::update::main(args).await,
  }
}
