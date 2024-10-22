use crate::arch::ArchContext;
use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::modules::VFS;
use crate::task::sched::SCHEDULER;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::{vec, vec::Vec};
use atomic::{Atomic, Ordering};
use core::any::Any;
use core::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, AtomicUsize};
use klib::proc::{MemSpace, Process, PID};
use klib::task::{RunState, Runnable, Task, TaskId};
use memory::page_table::PageTable;
use spin::Mutex;
use vfs::{Fd, VFSRequest};

use super::runnables::Idle;
use super::runnables::Init;
use super::sync::SysMonitor;

// impl Drop for MMState {
//     fn drop(&mut self) {
//         let guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
//         let user_page_table = unsafe { &mut *self.page_table.load(Ordering::SeqCst) };
//         if guard.deref() as *const PageTable != user_page_table as *const PageTable {
//             crate::memory::utils::release_user_page_table(user_page_table);
//         }
//     }
// }
static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct ProcessManager {
    procs: Mutex<BTreeMap<PID, Arc<Process>>>,
}

unsafe impl Sync for ProcessManager {}

pub static PROCESS_MANAGER: ProcessManager = ProcessManager::new();

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl ProcessManager {
    const fn new() -> Self {
        Self {
            procs: Mutex::new(BTreeMap::new()),
        }
    }

    fn new_mem_space(&self) -> Box<MemSpace> {
        Box::new(MemSpace {
            page_table: {
                // the initial page table is the kernel page table
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                AtomicPtr::new(PageTable::get())
            },
            has_user_page_table: AtomicBool::new(false),
            highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
        })
    }

    fn spawn_process(&self, runnable: impl Runnable + 'static) -> Arc<Process> {
        let pid = PID(COUNTER.fetch_add(1, Ordering::SeqCst));
        let fs = VFS.register_process(pid, "".to_owned());
        let proc = Arc::new(Process {
            id: pid,
            threads: Mutex::new(Vec::new()),
            mem: self.new_mem_space(),
            fs,
            monitor: Box::new(SysMonitor::new()),
            is_zombie: AtomicBool::new(false),
            exit_code: AtomicIsize::new(0),
        });
        // Create main thread
        let ctx = SCHEDULER.create_task_context();
        let runnable = Box::new(runnable);
        let task = self.create_task(proc.clone(), runnable, ctx);
        proc.threads.lock().push(task.id);
        // Add to list
        self.procs.lock().insert(proc.id, proc.clone());
        // Spawn
        SCHEDULER.register_new_task(task);
        proc
    }

    pub fn spawn_sched_process(&self) -> Arc<Process> {
        self.spawn_process(Idle)
    }

    pub fn spawn_init_process(&self) -> Arc<Process> {
        self.spawn_process(Init::new())
    }

    fn create_task(
        &self,
        proc: Arc<Process>,
        runnable: Box<dyn Runnable>,
        context: Box<dyn Any>,
    ) -> Arc<Task> {
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        let task = Arc::new(Task {
            id,
            pid: proc.id,
            state: Atomic::new(RunState::Ready),
            ticks: AtomicUsize::new(0),
            context,
            // proc: Arc::downgrade(&proc),
            // live: Lazy::new(|| Monitor::new(true)),
            // sched: SERVICE.scheduler().new_state(),
            runnable: Some(runnable),
        });
        // SCHEDULER.register_new_task(task.clone());
        task
    }

    // pub fn spawn(&self, t: Box<dyn Runnable>) -> Arc<Process> {
    //     // Process::create(t, SERVICE.create_mm_state())
    //     Arc::new(Process {
    //         id: 0,
    //         threads: Mutex::new(vec![]),
    //         runnable: t,
    //         mem: MMState::new(),
    //     })
    // }

    pub fn get_proc_by_id(&self, id: PID) -> Option<Arc<Process>> {
        self.procs.lock().get(&id).cloned()
    }

    pub fn current_proc(&self) -> Option<Arc<Process>> {
        self.current_proc_id()
            .and_then(|id| self.get_proc_by_id(id))
    }

    pub fn current_proc_id(&self) -> Option<PID> {
        SCHEDULER.get_current_task().map(|t| t.pid)
    }

    pub fn end_current_task(&self) {
        let task = SCHEDULER.get_current_task().unwrap();
        assert!(!interrupt::is_enabled());
        // Mark as dead
        // {
        //     let mut live = task.live.lock();
        //     *live = false;
        //     task.live.notify_all()
        // }
        // Remove from process
        let proc = self.get_proc_by_id(task.pid).unwrap();
        let mut tasks = proc.threads.lock();
        let index = tasks.iter().position(|t| *t == task.id).unwrap();
        tasks.swap_remove(index);
        // Remove from scheduler
        SCHEDULER.remove_task(task.id);
    }

    pub fn exit_current_proc(&self) {
        let _guard = interrupt::uninterruptible();
        let proc = self.current_proc().unwrap();
        // Release file handles
        VFS.deregister_process(proc.id);
        // Release memory
        // - Note: this is done in the MMState destructor
        // Mark as dead
        // {
        //     let mut live = self.live.lock();
        //     *live = false;
        //     self.live.notify_all();
        // }
        // Remove from scheduler
        let monitor = proc.monitor.as_ref().downcast_ref::<SysMonitor>().unwrap();
        monitor.lock();
        proc.is_zombie.store(true, Ordering::SeqCst);
        proc.exit_code.store(0, Ordering::SeqCst);
        monitor.notify_all();
        monitor.unlock();
        let threads = proc.threads.lock();
        for t in &*threads {
            SCHEDULER.remove_task(*t)
        }
        // Remove from procs
        self.procs.lock().remove(&proc.id);
    }

    pub fn fork(&self, proc: Arc<Process>) -> Arc<Process> {
        trace!("Forking process");
        let child_pid = PID(COUNTER.fetch_add(1, Ordering::SeqCst));
        let fs = VFS.fork_process(&proc, child_pid);
        let child = Arc::new(Process {
            id: child_pid,
            threads: Mutex::new(Vec::new()),
            mem: crate::memory::utils::fork_mem_space(&proc.mem),
            fs,
            monitor: Box::new(SysMonitor::new()),
            is_zombie: AtomicBool::new(false),
            exit_code: AtomicIsize::new(0),
        });
        trace!(
            "Created child process pid={:?} parent={:?}",
            child.id,
            proc.id
        );
        // Fork calling thread
        let current_task = SCHEDULER.get_current_task().unwrap();
        let tid = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        let ctx = <TargetArch as Arch>::Context::of(&current_task).fork();
        ctx.set_response_status(0);
        let main = Task {
            id: tid,
            pid: child.id,
            state: Atomic::new(RunState::Ready),
            ticks: AtomicUsize::new(current_task.ticks.load(Ordering::SeqCst)),
            context: Box::new(ctx),
            runnable: None,
        };
        child.threads.lock().push(tid);
        self.procs.lock().insert(child.id, child.clone());
        SCHEDULER.register_new_task(Arc::new(main));
        child
    }

    fn load_elf_for_exec(&self, path: &str) -> Result<Vec<u8>, ()> {
        let mut elf = vec![];
        let fd = crate::modules::module_call("vfs", false, &VFSRequest::Open(path));
        if fd < 0 {
            println!("load_elf_for_exec failed: fd={}", fd);
            return Err(());
        }
        let mut buf = [0u8; 256];
        loop {
            let size =
                crate::modules::module_call("vfs", false, &VFSRequest::Read(Fd(fd as _), &mut buf));
            if size > 0 {
                elf.extend_from_slice(&buf[0..size as usize]);
            } else if size < 0 {
                println!("load_elf_for_exec failed: size={}", size);
                return Err(());
            } else {
                break;
            }
        }
        crate::modules::module_call("vfs", false, &VFSRequest::Close(Fd(fd as _)));
        Ok(elf)
    }

    pub fn exec(&self, path: &str, _args: &[&str]) -> isize {
        let Ok(elf) = self.load_elf_for_exec(path) else {
            println!("exec failed: {}", path);
            return -1;
        };
        let proc = PROCESS_MANAGER.current_proc().unwrap();
        super::user::exec(proc, elf, &[])
    }
}
