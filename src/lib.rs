// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait)]

mod arb;
mod shrink;

pub use arb::*;
pub use shrink::*;

use std::ops::CoroutineState;
use std::pin::Pin;

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

pub fn falsify_with_rejections<T: Clone>(
    test: impl Fn(T) -> TestResult,
    mut root_arb_coro: impl ArbCoro<T> + Unpin,
    mut num_tests: usize,
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

pub fn falsify<T: Clone>(
    test: impl Fn(T) -> bool,
    root_arb_coro: impl ArbCoro<T> + Unpin,
    num_tests: usize,
) -> Option<T> {
    falsify_with_rejections(
        |value| TestResult::from(test(value)),
        root_arb_coro,
        num_tests,
    )
}

#[cfg(test)]
mod tests {
    use crate::shrink::{shrink, shrink_usize_exhaustive};

    use super::*;
    use rand::SeedableRng;
    use rand::rngs::{StdRng, SysRng};
    use std::assert_matches;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_arb_usize() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        assert_matches!(falsify(|n| n % 2 == 0 || n % 2 == 1, arb, 100), None);
    }

    #[test]
    fn test_arb_vec_usize() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb_usize = arb_usize(rng.clone());
        let arb_vec = arb_vec(arb_usize, rng, 50);
        assert_matches!(
            falsify(
                |mut v| {
                    let original = v.clone();
                    v.reverse();
                    v.reverse();
                    v == original
                },
                arb_vec,
                100,
            ),
            None
        );
    }

    #[test]
    fn test_shrink() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        let test = |n| n % 2 == 0;
        if let Some(falsifier) = falsify(test, arb, 100) {
            let shrink_strategy = shrink_usize_exhaustive(falsifier);
            shrink(test, shrink_strategy);
        };
    }
}
