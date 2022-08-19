use alloc::{boxed::Box, string::String};
use proc::ProcId;
use vfs::{ramfs::RamFS, FileSystem};

static mut VFS_IMPL: &'static dyn vfs::VFSManager = &UnimplementedFSManager;

pub static VFS: FSManager = FSManager;

pub struct FSManager;

impl FSManager {
    pub fn get_vfs_manager(&self) -> &'static dyn vfs::VFSManager {
        unsafe { VFS_IMPL }
    }

    pub fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager) {
        unsafe {
            VFS_IMPL = vfs_manager;
        }
    }

    pub fn init(&self, ramfs: &'static mut RamFS) {
        unsafe { VFS_IMPL.init(ramfs) }
    }

    pub fn register_process(&self, proc: ProcId, cwd: String) -> Box<dyn core::any::Any> {
        unsafe { VFS_IMPL.register_process(proc, cwd) }
    }

    pub fn deregister_process(&self, proc: ProcId) {
        unsafe { VFS_IMPL.deregister_process(proc) }
    }
}

struct UnimplementedFSManager;

impl vfs::VFSManager for UnimplementedFSManager {
    fn init(&self, _ramfs: &'static mut RamFS) {
        unimplemented!()
    }
    fn register_process(&self, _proc: ProcId, _cwd: String) -> Box<dyn core::any::Any> {
        unimplemented!()
    }
    fn deregister_process(&self, _proc: ProcId) {
        unimplemented!()
    }
    fn register_fs(&self, _fs: &'static dyn FileSystem) {
        unimplemented!()
    }
}
