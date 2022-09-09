use core::ops::Deref;

static mut PROCESS_MANAGER_IMPL: Option<&'static dyn proc::ProcessManager> = None;

pub static PROCESS_MANAGER: ProcessManager = ProcessManager;

pub struct ProcessManager;

impl ProcessManager {
    pub fn set_process_manager(&self, pm: &'static dyn proc::ProcessManager) {
        unsafe { PROCESS_MANAGER_IMPL = Some(pm) }
    }
}

impl Deref for ProcessManager {
    type Target = dyn proc::ProcessManager;
    fn deref(&self) -> &Self::Target {
        unsafe { PROCESS_MANAGER_IMPL.unwrap_unchecked() }
    }
}
