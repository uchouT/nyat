use super::{MappingHandler, MappingInfo};
use std::process::{Child, Command, Stdio};
pub(super) struct ExecHook {
    cmd: String,
    children: Vec<Child>,
}

impl ExecHook {
    fn reap(&mut self) {
        self.children
            .retain_mut(|c| c.try_wait().ok().flatten().is_none());
    }

    pub(super) fn new(cmd: String) -> Self {
        Self {
            cmd,
            children: Vec::with_capacity(4),
        }
    }
}

impl MappingHandler for ExecHook {
    fn on_change(&mut self, info: MappingInfo) {
        self.reap();
        match Command::new("sh")
            .arg("-c")
            .arg(&self.cmd)
            .env("NYAT_PUB_ADDR", info.pub_addr.ip().to_string())
            .env("NYAT_PUB_PORT", info.pub_addr.port().to_string())
            .env("NYAT_LOCAL_ADDR", info.local_addr.ip().to_string())
            .env("NYAT_LOCAL_PORT", info.local_addr.port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
        {
            Ok(child) => self.children.push(child),
            Err(e) => eprintln!("nyat: exec failed: {e}"),
        }
    }
}
