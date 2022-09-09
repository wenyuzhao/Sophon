use core::ops::Deref;

static mut SCHEDULER_IMPL: Option<&'static dyn sched::Scheduler> = None;

pub static SCHEDULER: Scheduler = Scheduler;

pub struct Scheduler;

impl Scheduler {
    pub fn set_scheduler(&self, scheduler: &'static dyn sched::Scheduler) {
        unsafe {
            SCHEDULER_IMPL = Some(scheduler);
        }
    }
}

impl Deref for Scheduler {
    type Target = dyn sched::Scheduler;
    fn deref(&self) -> &Self::Target {
        unsafe { SCHEDULER_IMPL.unwrap_unchecked() }
    }
}
