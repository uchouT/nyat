mod cli;
use clap::Parser;

fn main() -> Result<(), anyhow::Error> {
    let config = cli::Config::parse();
    Ok(())
}
