use clap::{CommandFactory, Parser};

#[derive(Parser)]
#[command(
  name = "spec-store",
  version,
  about = "Remote, multi-project store for Spec Driven Development documents"
)]
struct Cli {}

fn main() {
  Cli::parse();
  Cli::command().print_help().expect("help output");
}
