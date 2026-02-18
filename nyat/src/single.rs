use nyat_core::mapper::{Mapper, MappingHandler};
use std::io::Write;
use std::time::Duration;

struct Handler;

impl MappingHandler for Handler {
    fn on_change(&mut self, info: nyat_core::mapper::MappingInfo) {
        if writeln!(
            std::io::stdout(),
            "{} {} {} {}",
            info.pub_addr.ip(),
            info.pub_addr.port(),
            info.local_addr.ip(),
            info.local_addr.port(),
        )
        .is_err()
        {
            std::process::exit(0);
        }
    }
}

pub fn proc(mapper: Mapper) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        loop {
            match mapper.run(&mut Handler).await {
                Ok(()) => {}
                Err(e) if e.is_recoverable() => {
                    eprintln!("nyat: {:#}, retrying...", anyhow::Error::from(e));
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                Err(e) => return Err(anyhow::Error::from(e)),
            }
        }
    })
}
