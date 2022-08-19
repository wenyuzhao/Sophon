use core::ops::Deref;

static mut VFS_IMPL: Option<&'static dyn vfs::VFSManager> = None;

pub static VFS: VFSManager = VFSManager;

pub struct VFSManager;

impl VFSManager {
    pub fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager) {
        unsafe {
            VFS_IMPL = Some(vfs_manager);
        }
    }
}

impl Deref for VFSManager {
    type Target = dyn vfs::VFSManager;
    fn deref(&self) -> &Self::Target {
        unsafe { VFS_IMPL.unwrap_unchecked() }
    }
}
