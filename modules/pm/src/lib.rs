#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![feature(generic_associated_types)]
#![feature(downcast_unchecked)]
#![feature(drain_filter)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

mod locks;

use alloc::{boxed::Box, vec::Vec};
use atomic::{Atomic, Ordering};
use core::{any::Any, sync::atomic::AtomicUsize};
use crossbeam::queue::SegQueue;
use kernel_module::{kernel_module, KernelModule, ProcessorLocalStorage, SERVICE};
use locks::{RawCondvar, RawMutex};
use proc::{ProcId, TaskId};
use sched::{RunState, Scheduler};
use spin::{Lazy, Mutex};
use syscall::module_calls::proc::ProcRequest;

const UNIT_TIME_SLICE: usize = 1;

#[derive(Debug, Default)]
pub struct State {
    locks: Mutex<Vec<*mut RawMutex>>,
    cvars: Mutex<Vec<*mut RawCondvar>>,
}

#[kernel_module]
pub static mut PM: ProcessManager = ProcessManager::new();

pub struct ProcessManager {
    current_task: Vec<Atomic<Option<TaskId>>>,
    per_core_task_queue: Lazy<ProcessorLocalStorage<SegQueue<TaskId>>>,
}

impl ProcessManager {
    const fn new() -> Self {
        Self {
            current_task: Vec::new(),
            per_core_task_queue: Lazy::new(|| ProcessorLocalStorage::new()),
        }
    }

    #[inline]
    fn get_state(&self, proc: ProcId) -> &State {
        let state = SERVICE.get_vfs_state(proc);
        unsafe { state.downcast_ref_unchecked::<State>() }
    }
}

impl proc::ProcessManager for ProcessManager {
    fn new_state(&self) -> Box<dyn Any> {
        Box::new(State::default())
    }
}

impl KernelModule for ProcessManager {
    type ModuleRequest<'a> = ProcRequest;

    fn init(&'static mut self) -> anyhow::Result<()> {
        self.current_task
            .resize_with(SERVICE.num_cores(), || Atomic::new(None));
        SERVICE.set_process_manager(self);
        Ok(())
    }

    fn handle_module_call<'a>(&self, privileged: bool, request: Self::ModuleRequest<'a>) -> isize {
        match request {
            ProcRequest::MutexCreate => {
                let mutex = Box::leak(box RawMutex::new()) as *mut RawMutex;
                if !privileged {
                    let proc = SERVICE.current_process().unwrap();
                    self.get_state(proc).locks.lock().push(mutex);
                }
                mutex as _
            }
            ProcRequest::MutexLock(mutex) => {
                let mutex = mutex.cast::<RawMutex>();
                mutex.lock();
                0
            }
            ProcRequest::MutexUnlock(mutex) => {
                let mutex = mutex.cast::<RawMutex>();
                mutex.unlock();
                0
            }
            ProcRequest::MutexDestroy(mutex) => {
                let mutex = mutex.cast_mut_ptr::<RawMutex>();
                let proc = SERVICE.current_process().unwrap();
                let mut locks = self.get_state(proc).locks.lock();
                if locks.drain_filter(|x| *x == mutex).count() > 0 {
                    let _boxed = unsafe { Box::from_raw(mutex) };
                }
                0
            }
            ProcRequest::CondvarCreate => {
                let cvar = Box::leak(box RawCondvar::new()) as *mut RawCondvar;
                if !privileged {
                    let proc = SERVICE.current_process().unwrap();
                    self.get_state(proc).cvars.lock().push(cvar);
                }
                cvar as _
            }
            ProcRequest::CondvarWait(cvar, mutex) => {
                let cvar = cvar.cast::<RawCondvar>();
                let mutex = mutex.cast::<RawMutex>();
                cvar.wait(mutex);
                0
            }
            ProcRequest::CondvarNotifyAll(cvar) => {
                let cvar = cvar.cast::<RawCondvar>();
                cvar.notify_all();
                0
            }
            ProcRequest::CondvarDestroy(cvar) => {
                let cvar = cvar.cast_mut_ptr::<RawCondvar>();
                let proc = SERVICE.current_process().unwrap();
                let mut cvars = self.get_state(proc).cvars.lock();
                if cvars.drain_filter(|x| *x == cvar).count() > 0 {
                    let _boxed = unsafe { Box::from_raw(cvar) };
                }
                0
            }
        }
    }
}
