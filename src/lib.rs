// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait)]

mod arb;
mod shrink;

pub use arb::*;
pub use shrink::*;

use std::cell::RefCell;
use std::ops::CoroutineState;
use std::pin::Pin;
use std::rc::Rc;

#[derive(Copy, Clone)]
pub enum TestResult {
    Pass,
    Fail,
    Reject,
}

impl From<bool> for TestResult {
    fn from(cond: bool) -> Self {
        if cond {
            TestResult::Pass
        } else {
            TestResult::Fail
        }
    }
}

pub fn guess_falsifier<T: Clone>(
    mut root_arb_coro: impl ArbCoro<T> + Unpin,
    mut num_tests: usize,
    test: impl Fn(T) -> TestResult,
) -> Option<T> {
    while num_tests > 0 {
        match Pin::new(&mut root_arb_coro).resume(()) {
            CoroutineState::Yielded(val) => match test(val.clone()) {
                TestResult::Pass => {
                    num_tests -= 1;
                    continue;
                }
                TestResult::Reject => continue,
                TestResult::Fail => return Some(val),
            },
            CoroutineState::Complete(()) => panic!("generator should produce values indefinitely"),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::shrink::{shrink, shrink_usize_exhaustive};

    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rand::rngs::SysRng;
    use std::assert_matches;

    #[test]
    fn frob() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        assert_matches!(
            guess_falsifier(arb, 100, |n| TestResult::from(n % 2 == 0 || n % 2 == 1)),
            None
        );
    }

    #[test]
    fn frob_vec() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb_usize = arb_usize(rng.clone());
        let arb_vec = arb_vec(arb_usize, rng, 50);
        assert_matches!(
            guess_falsifier(arb_vec, 100, |mut v| {
                let original = v.clone();
                v.reverse();
                v.reverse();
                TestResult::from(v == original)
            }),
            None
        );
    }

    #[test]
    fn test_shrinking() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        let test = |n| TestResult::from(dbg!(n) % 2 == 0);
        if let Some(falsifier) = guess_falsifier(arb, 100, test) {
            let shrink_strategy = shrink_usize_exhaustive(falsifier);
            shrink(shrink_strategy, test);
        };
    }
}
