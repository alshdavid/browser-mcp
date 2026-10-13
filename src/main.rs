mod cmd;
mod mcp;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  match Command::parse().command {
    Commands::Stdio(args) => cmd::stdio::main(args).await,
  }
}
