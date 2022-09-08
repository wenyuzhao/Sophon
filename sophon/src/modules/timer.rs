use core::ops::Deref;

pub static mut TIMER_IMPL: Option<&'static dyn interrupt::TimerController> = None;

pub static TIMER: TimerController = TimerController;

pub struct TimerController;

impl TimerController {
    pub fn set_timer_controller(&self, timer: &'static dyn interrupt::TimerController) {
        unsafe { TIMER_IMPL = Some(timer) }
    }
}

impl Deref for TimerController {
    type Target = dyn interrupt::TimerController;
    fn deref(&self) -> &Self::Target {
        unsafe { TIMER_IMPL.unwrap_unchecked() }
    }
}
