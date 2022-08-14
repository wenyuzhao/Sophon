use spin::RwLock;

pub use testing::{Test, Tests};

use crate::{
    arch::{Arch, TargetArch},
    task::{runnable::Runnable, Proc},
};

static BOOT_TESTS: RwLock<Tests> = RwLock::new(Tests::new("boot"));

static KERNEL_TESTS: RwLock<Tests> = RwLock::new(Tests::new("kernel"));

pub enum TestKind {
    Boot,
    Kernel,
}

pub fn register_test(kind: TestKind, test: &'static dyn Test) {
    let mut tests = match kind {
        TestKind::Boot => BOOT_TESTS.write(),
        TestKind::Kernel => KERNEL_TESTS.write(),
    };
    tests.add(test);
}

pub fn register_kernel_tests(tests: Tests) {
    KERNEL_TESTS.write().merge(tests)
}

pub fn run_boot_tests() {
    assert!(cfg!(sophon_test));
    BOOT_TESTS.read().run();
}

pub fn start_kernel_test_runner() {
    assert!(cfg!(sophon_test));
    let _proc = Proc::spawn(box KernelTestRunner);
}

pub struct KernelTestRunner;

impl Runnable for KernelTestRunner {
    fn run(&mut self) -> ! {
        assert!(cfg!(sophon_test));
        KERNEL_TESTS.read().run();
        TargetArch::halt(0)
    }
}
