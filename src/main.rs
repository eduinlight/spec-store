use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
  let cli = spec_store::cli::Cli::parse();
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_writer(std::io::stderr)
    .init();
  if let Err(error) = spec_store::cli::run(cli) {
    eprintln!("error: {error}");
    std::process::exit(1);
  }
}
