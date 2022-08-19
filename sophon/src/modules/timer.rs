pub static mut TIMER_IMPL: &'static dyn interrupt::TimerController = &UnimplementedTimerController;

pub static TIMER: TimerController = TimerController;

pub struct TimerController;

impl TimerController {
    pub fn init(&self, bsp: bool) {
        unsafe { TIMER_IMPL.init(bsp) }
    }
    pub fn get_timer_controller(&self) -> &'static dyn interrupt::TimerController {
        unsafe { TIMER_IMPL }
    }
    pub fn set_timer_controller(&self, timer: &'static dyn interrupt::TimerController) {
        unsafe { TIMER_IMPL = timer }
    }
}

struct UnimplementedTimerController;

impl interrupt::TimerController for UnimplementedTimerController {
    fn init(&self, _bsp: bool) {
        unimplemented!()
    }
}
