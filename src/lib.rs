// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

mod arb;
mod shrink;
mod test_tree_arena;

pub use arb::*;
pub use shrink::*;

use std::ops::CoroutineState;
use std::panic::{AssertUnwindSafe, catch_unwind};
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
    mut tries: usize,
) -> Option<T> {
    while tries > 0 {
        let value = match Pin::new(&mut root_arb_coro).resume(()) {
            CoroutineState::Yielded(value) => value,
            CoroutineState::Complete(()) => panic!("generator should produce values indefinitely"),
        };
        let result = catch_unwind(AssertUnwindSafe(|| test(value.clone())));
        match result {
            Ok(TestResult::Pass) => {
                tries -= 1;
                continue;
            }
            Ok(TestResult::Reject) => continue,
            Ok(TestResult::Fail) | Err(..) => return Some(value),
        }
    }
    None
}

pub fn falsify<T: Clone>(
    test: impl Fn(T) -> bool,
    root_arb_coro: impl ArbCoro<T> + Unpin,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), root_arb_coro, 100)
}

#[cfg(test)]
mod tests {
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
        assert_matches!(falsify(|n| n % 2 == 0 || n % 2 == 1, arb), None);
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
            ),
            None
        );
    }

    #[test]
    fn test_shrink() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        let test = |n| n % 2 == 0;
        if let Some(falsifier) = falsify(test, arb) {
            let shrink_strategy = shrink_usize_exhaustive(falsifier);
            shrink(test, shrink_strategy);
        };
    }
}
