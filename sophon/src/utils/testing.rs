use spin::RwLock;

use crate::{
    arch::{Arch, TargetArch},
    task::{runnable::Runnable, Proc},
};

const MAX_TESTS: usize = 128;

struct Tests {
    tests: [Option<&'static dyn Test>; MAX_TESTS],
    len: usize,
}

impl Tests {
    const fn new() -> Self {
        Tests {
            tests: [None; MAX_TESTS],
            len: 0,
        }
    }

    fn add(&mut self, test: &'static dyn Test) {
        let index = self.len;
        self.tests[index] = Some(test);
        self.len += 1;
    }
}

static BOOT_TESTS: RwLock<Tests> = RwLock::new(Tests::new());

static KERNEL_TESTS: RwLock<Tests> = RwLock::new(Tests::new());

pub enum TestKind {
    Boot,
    Kernel,
}

pub trait Test: Sync {
    fn run(&self) -> ();
}

impl<T: Fn() + Sync> Test for T {
    fn run(&self) {
        let name = core::any::type_name::<T>().rsplit_once("::").unwrap().0;
        print!("{}...\t", name);
        self();
        println!("[ok]");
    }
}

pub fn register_test(kind: TestKind, test: &'static dyn Test) {
    let mut tests = match kind {
        TestKind::Boot => BOOT_TESTS.write(),
        TestKind::Kernel => KERNEL_TESTS.write(),
    };
    tests.add(test);
}

fn run_tests(name: &str, kind: TestKind) {
    assert!(cfg!(sophon_test));
    let tests = match kind {
        TestKind::Boot => BOOT_TESTS.read(),
        TestKind::Kernel => KERNEL_TESTS.read(),
    };
    println!("\n---");
    println!("Running {} {} tests", tests.len, name);
    for i in 0..tests.len {
        let test = tests.tests[i].unwrap();
        test.run();
    }
    println!("All tests passed.");
    println!("---\n");
}

pub fn run_boot_tests() {
    assert!(cfg!(sophon_test));
    run_tests("boot", TestKind::Boot);
}

pub fn start_kernel_test_runner() {
    assert!(cfg!(sophon_test));
    let _proc = Proc::spawn(box KernelTestRunner);
}

pub struct KernelTestRunner;

impl Runnable for KernelTestRunner {
    fn run(&mut self) -> ! {
        assert!(cfg!(sophon_test));
        run_tests("kernel", TestKind::Kernel);
        TargetArch::halt(0)
    }
}
