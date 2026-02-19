mod handle;
mod parse;
use anyhow::Result;
use parse::MultiConfig;
use std::path::PathBuf;

pub fn proc(path: PathBuf) -> Result<()> {
    let config = MultiConfig::load(&path)?;
    handle::run(config)?;
    Ok(())
}
