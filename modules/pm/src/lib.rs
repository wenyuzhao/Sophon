#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![feature(generic_associated_types)]
#![feature(downcast_unchecked)]
#![feature(drain_filter)]
#![feature(const_btree_new)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

mod locks;
mod proc;
mod task;

use ::proc::{Proc, ProcId, Runnable, TaskId};
use alloc::{boxed::Box, sync::Arc};
use core::any::Any;
use kernel_module::{kernel_module, KernelModule, SERVICE};
use locks::{RawCondvar, RawMutex};
use syscall::module_calls::proc::ProcRequest;

use crate::proc::Process;

#[kernel_module]
pub static mut PM: ProcessManager = ProcessManager;

pub struct ProcessManager;

impl ::proc::ProcessManager for ProcessManager {
    fn spawn(&self, t: Box<dyn Runnable>, mm: Box<dyn Any>) -> Arc<dyn Proc> {
        let proc = Process::create(t, mm);
        proc
    }
    fn get_proc_by_id(&self, id: ProcId) -> Option<Arc<dyn Proc>> {
        Process::by_id(id).map(|p| p.as_dyn())
    }
    fn current_proc(&self) -> Option<Arc<dyn Proc>> {
        Process::current().map(|p| p.as_dyn())
    }
    fn current_proc_id(&self) -> Option<ProcId> {
        self.current_proc().map(|p| p.id())
    }
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<dyn ::proc::Task>> {
        task::Task::by_id(id).map(|t| t.as_dyn())
    }
    fn current_task(&self) -> Option<Arc<dyn ::proc::Task>> {
        task::Task::current().map(|t| t.as_dyn())
    }
}

impl KernelModule for ProcessManager {
    type ModuleRequest<'a> = ProcRequest;

    fn init(&'static mut self) -> anyhow::Result<()> {
        SERVICE.set_process_manager(self);
        Ok(())
    }

    fn handle_module_call<'a>(&self, privileged: bool, request: Self::ModuleRequest<'a>) -> isize {
        match request {
            ProcRequest::MutexCreate => {
                let mutex = Box::leak(box RawMutex::new()) as *mut RawMutex;
                if !privileged {
                    Process::current().unwrap().locks.lock().push(mutex);
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
                let proc = Process::current().unwrap();
                let mut locks = proc.locks.lock();
                if locks.drain_filter(|x| *x == mutex).count() > 0 {
                    let _boxed = unsafe { Box::from_raw(mutex) };
                }
                0
            }
            ProcRequest::CondvarCreate => {
                let cvar = Box::leak(box RawCondvar::new()) as *mut RawCondvar;
                if !privileged {
                    Process::current().unwrap().cvars.lock().push(cvar);
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
                let proc = Process::current().unwrap();
                let mut cvars = proc.cvars.lock();
                if cvars.drain_filter(|x| *x == cvar).count() > 0 {
                    let _boxed = unsafe { Box::from_raw(cvar) };
                }
                0
            }
        }
    }
}
