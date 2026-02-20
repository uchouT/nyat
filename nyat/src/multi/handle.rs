use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use nyat_core::mapper::{Mapper, MappingHandler, MappingInfo};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

use crate::hooks::Hooks;

struct TaskHandler {
    hooks: Hooks,
    name: String,
}

impl TaskHandler {
    fn new(name: String, hooks: Hooks) -> Self {
        Self { hooks, name }
    }
}

impl MappingHandler for TaskHandler {
    fn on_change(&mut self, info: MappingInfo) {
        self.hooks.on_change(info);

        let _ = writeln!(
            std::io::stdout(),
            "[{}] {} {} {} {}",
            self.name,
            info.pub_addr.ip(),
            info.pub_addr.port(),
            info.local_addr.ip(),
            info.local_addr.port(),
        );
    }
}

async fn run_task(mapper: Mapper, handler: &mut TaskHandler) {
    loop {
        match mapper.run(handler).await {
            Ok(()) => {}
            Err(e) if e.is_recoverable() => {
                eprintln!(
                    "[{}] {:#}, retrying...",
                    handler.name,
                    anyhow::Error::from(e)
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                eprintln!("[{}] fatal: {:#}", handler.name, anyhow::Error::from(e));
                break;
            }
        }
    }
}

pub(super) fn run(multi_config: super::MultiConfig) -> Result<()> {
    let rt = Runtime::new()?;

    rt.block_on(async {
        let mut set = JoinSet::new();

        for (name, mut config) in multi_config.tasks {
            let exec = config.exec.take();
            let mapper = config.into_mapper();
            let mut handler = TaskHandler::new(name, Hooks::new(exec));
            set.spawn(async move {
                run_task(mapper, &mut handler).await;
            });
        }

        while let Some(result) = set.join_next().await {
            if let Err(e) = result {
                eprintln!("task panicked: {e}");
            }
        }
    });

    Ok(())
}
