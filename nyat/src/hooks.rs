mod exec;

use exec::ExecHook;
use nyat_core::mapper::{MappingHandler, MappingInfo};

pub(crate) struct Hooks {
    exec: Option<ExecHook>,
}

impl Hooks {
    pub fn new(exec: Option<String>) -> Self {
        Self {
            exec: exec.map(ExecHook::new),
        }
    }
}

impl MappingHandler for Hooks {
    fn on_change(&mut self, info: MappingInfo) {
        if let Some(exec) = &mut self.exec {
            exec.on_change(info);
        }
    }
}
