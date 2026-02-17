mod cli;
mod single;

use cli::Config;

fn main() -> anyhow::Result<()> {
    match Config::parse() {
        Config::Single(mapper) => single::proc(mapper)?,
        Config::Multi(path) => todo!(),
    }
    Ok(())
}
