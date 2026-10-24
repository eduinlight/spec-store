use clap::Parser;
use sdd_vault_cli::cli::{Cli, run};

#[tokio::main]
async fn main() {
  let cli = Cli::parse();
  if let Err(error) = run(cli).await {
    eprintln!("error: {error}");
    std::process::exit(1);
  }
}
