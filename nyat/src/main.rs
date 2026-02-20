mod cli;
mod config;
mod hooks;
mod multi;
mod single;

use cli::Config;

fn main() -> anyhow::Result<()> {
    match Config::parse() {
        Config::Single(config) => single::proc(config)?,
        Config::Multi(path) => multi::proc(path)?,
    }
    Ok(())
}
