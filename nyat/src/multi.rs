mod parse;
use anyhow::Result;
use parse::MultiConfig;
use std::path::PathBuf;

pub fn proc(path: PathBuf) -> Result<()> {
    let _config = MultiConfig::load(&path)?;
    todo!("spawn and run tasks concurrently")
}
