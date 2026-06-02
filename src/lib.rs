// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait)]

use rand::{
    RngExt, SeedableRng,
    rngs::{StdRng, SysRng},
};
use std::cell::RefCell;
use std::ops::Range;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::rc::Rc;

// TODO Add type alias/trait: TestCaseCoro = Coroutine<Return = ()>

pub struct TestDriver {
    rng: Rc<RefCell<StdRng>>,
}

impl TestDriver {
    fn from_random_seed() -> Self {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        Self { rng }
    }

    pub fn from_seed(seed: u64) -> Self {
        let rng = Rc::new(RefCell::new(StdRng::seed_from_u64(seed)));
        Self { rng }
    }

    pub fn drive<C, Value, R, F>(mut test_case_generator: Box<C>, satisfied: F) -> bool
    where
        C: Coroutine<Yield = Value, Return = ()> + Unpin,
        F: Fn(Value) -> bool,
    {
        loop {
            match Pin::new(&mut test_case_generator).resume(()) {
                CoroutineState::Yielded(val) => {
                    if !satisfied(val) {
                        return false;
                    }
                }
                CoroutineState::Complete(_) => return true,
            }
        }
    }
}

// TODO drive() takes a coroutine type directly so this trait probably isn't necessary
pub trait TestCaseGenerator {
    fn make_gen(&self, size: usize, rng: Rc<RefCell<StdRng>>) -> impl Coroutine;
}

// TODO Confusing mismatch: TestCaseGenerator implemented for a range but generates usizes
// TODO Instead of impl on a Range, take min and max as either constructor args or fn args.
impl TestCaseGenerator for Range<usize> {
    fn make_gen(
        &self,
        num_test_cases: usize,
        rng: Rc<RefCell<StdRng>>,
    ) -> impl Coroutine<Yield = usize, Return = ()> {
        #[coroutine]
        move || {
            for _ in 0..num_test_cases {
                let value = {
                    let mut r = rng.borrow_mut();
                    let v = r.random_range(self.clone());
                    v
                };
                yield value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frob() {
        // TODO Take rng out of TestDriver and ditch TestDriver for simplicity
        let driver = TestDriver::from_random_seed();
        let generator = (0..100).make_gen(100, driver.rng);
        TestDriver::drive::<_, usize, (), _>(Box::new(generator), |n| n % 2 == 0 || n % 2 == 1);
    }
}
