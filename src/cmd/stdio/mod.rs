#[derive(Debug, clap::Parser)]
pub struct StdioCommand {}

pub async fn main(_args: StdioCommand) -> anyhow::Result<()> {
  Ok(())
}
