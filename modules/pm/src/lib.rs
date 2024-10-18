#![feature(format_args_nl)]
#![feature(downcast_unchecked)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

mod locks;
mod proc;

use ::proc::{Proc, ProcId, Runnable, Task, TaskId};
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use locks::{RawCondvar, RawMutex};
use spin::Mutex;
use syscall::module_calls::proc::ProcRequest;

use crate::proc::Process;

#[kernel_module]
pub static mut PM: ProcessManager = ProcessManager;

pub struct ProcessManager;

static TASKS: Mutex<BTreeMap<TaskId, Arc<Task>>> = Mutex::new(BTreeMap::new());

impl ::proc::ProcessManager for ProcessManager {
    fn spawn(&self, t: Box<dyn Runnable>) -> Arc<dyn Proc> {
        Process::create(t, SERVICE.create_mm_state())
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
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<Task>> {
        TASKS.lock().get(&id).cloned()
    }
    fn current_task(&self) -> Option<Arc<Task>> {
        self.get_task_by_id(SERVICE.scheduler().get_current_task_id()?)
    }
    fn end_current_task(&self) {
        let task = self.current_task().unwrap();
        assert!(!interrupt::is_enabled());
        // Mark as dead
        {
            let mut live = task.live.lock();
            *live = false;
            task.live.notify_all()
        }
        // Remove from process
        let proc = task.proc.upgrade().unwrap().clone();
        let mut tasks = proc.tasks().lock();
        let index = tasks.iter().position(|t| *t == task.id).unwrap();
        tasks.swap_remove(index);
        // Remove from scheduler
        SERVICE.scheduler().remove_task(task.id);
        // Remove from all tasks
        TASKS.lock().remove(&task.id).unwrap();
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
                let mutex = Box::leak(Box::new(RawMutex::new())) as *mut RawMutex;
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
                if let Some(index) = locks.iter().position(|x| *x == mutex) {
                    locks.swap_remove(index);
                    let _boxed = unsafe { Box::from_raw(mutex) };
                }
                0
            }
            ProcRequest::CondvarCreate => {
                let cvar = Box::leak(Box::new(RawCondvar::new())) as *mut RawCondvar;
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
                if let Some(index) = cvars.iter().position(|x| *x == cvar) {
                    cvars.swap_remove(index);
                    let _boxed = unsafe { Box::from_raw(cvar) };
                }
                0
            }
        }
    }
}
