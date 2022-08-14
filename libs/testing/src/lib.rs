#![no_std]
#![feature(format_args_nl)]

use spin::RwLock;

#[macro_use]
extern crate log;

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

const MAX_TESTS: usize = 128;

pub struct Tests {
    pub name: &'static str,
    pub tests: [Option<&'static dyn Test>; MAX_TESTS],
    pub len: usize,
}

impl Tests {
    pub const fn new(name: &'static str) -> Self {
        Tests {
            name,
            tests: [None; MAX_TESTS],
            len: 0,
        }
    }

    pub fn add(&mut self, test: &'static dyn Test) {
        let index = self.len;
        self.tests[index] = Some(test);
        self.len += 1;
    }

    pub fn run_tests(&self) {
        assert!(cfg!(sophon_test));
        println!("\n--- Running {} {} tests ---", self.len, self.name);
        for i in 0..self.len {
            self.tests[i].unwrap().run();
        }
        println!("--- All tests passed ---\n");
    }

    pub fn merge(&mut self, tests: Tests) {
        for i in 0..tests.len {
            self.add(tests.tests[i].unwrap());
        }
    }
}

pub static TESTS: RwLock<Tests> = RwLock::new(Tests::new("kernel"));

pub fn register_test(test: &'static dyn Test) {
    TESTS.write().add(test);
}
