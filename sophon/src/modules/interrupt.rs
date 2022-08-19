// static INTERRUPT_CONTROLLER:

// UnimplementedInterruptController

// pub struct InterruptController;

// impl InterruptController {
//     pub fn new() -> Self {
//         Self {}
//     }
// }

// {
//     fn init(&self);
//     fn get_active_irq(&self) -> Option<usize>;
//     fn enable_irq(&self, irq: usize);
//     fn disable_irq(&self, irq: usize);
//     fn interrupt_begin(&self);
//     fn interrupt_end(&self);
//     fn get_irq_handler(&self, irq: usize) -> Option<&IRQHandler>;
//     fn set_irq_handler(&self, irq: usize, handler: IRQHandler);
// }

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
