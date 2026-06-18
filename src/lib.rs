// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

mod arb;
mod shrink;
mod test_tree_arena;
mod test_tree_refs;

pub use arb::*;
use rand::rngs::StdRng;
use rand::{SeedableRng, TryRng};
pub use shrink::*;

use std::cell::RefCell;
use std::ops::CoroutineState;
use std::panic::{RefUnwindSafe, catch_unwind};
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

pub fn falsify_with_rejections<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> TestResult + RefUnwindSafe,
    mut arb_t: impl ArbCoro<T> + Unpin,
    mut tries: usize,
) -> Option<T> {
    while tries > 0 {
        let value = match Pin::new(&mut arb_t).resume(()) {
            CoroutineState::Yielded(value) => value,
            CoroutineState::Complete(()) => panic!("generator should produce values indefinitely"),
        };
        let result = catch_unwind(|| test(value.clone()));
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

pub fn make_rng() -> Rc<RefCell<StdRng>> {
    let mut std_rng: StdRng = rand::make_rng();
    let seed = std_rng.try_next_u64().unwrap();
    dbg!(seed);
    let seeded_std_rng = StdRng::seed_from_u64(seed);
    Rc::new(RefCell::new(seeded_std_rng))
}

pub fn make_seeded_rng(seed: u64) -> Rc<RefCell<StdRng>> {
    let seeded_std_rng = StdRng::seed_from_u64(seed);
    Rc::new(RefCell::new(seeded_std_rng))
}

pub fn falsify<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> bool + RefUnwindSafe,
    arb_t: impl ArbCoro<T> + Unpin,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), arb_t, 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_arb_usize() {
        let rng = make_rng();
        let arb = arb_usize(rng);
        assert_matches!(falsify(|n| n % 2 == 0 || n % 2 == 1, arb), None);
    }

    #[test]
    fn test_arb_vec_of_usize() {
        let rng = make_rng();
        let arb_usize = arb_usize(rng.clone());
        let arb_vec = arb_vec_of(arb_usize, rng, 50);
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
        let rng = make_rng();
        let arb = arb_usize(rng);
        let test = |n| n % 2 == 0;
        if let Some(falsifier) = falsify(test, arb) {
            let shrink_strategy = shrink_usize_exhaustive(falsifier);
            shrink(test, shrink_strategy);
        };
    }
}
