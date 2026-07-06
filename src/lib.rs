//! Simple usage example (wrap it in a test):
//!
//! ```
//! use falsify::*;
//!
//! let rng = make_test_rng();
//! let arb = arb_usize(rng);
//! let property = |n| n < 10;
//! if let Some(falsifier) = falsify(property, arb) {
//!     let shrinker = shrink_usize_binary_search(falsifier);
//!     let smallest_falsifier = shrink(property, shrinker);
//!     assert_eq!(smallest_falsifier, 10);
//! }
//! ```

// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

mod arb;
mod shrinking;
mod sip;
mod test_tree_indexes;
mod test_tree_refs;

pub use arb::*;
use rand::rngs::StdRng;
use rand::{SeedableRng, TryRng};
pub use shrinking::*;
pub use sip::HasherBuilder;

use std::cell::RefCell;
use std::env;
use std::ops::{Coroutine, CoroutineState};
use std::panic::{RefUnwindSafe, catch_unwind};
use std::pin::pin;
use std::rc::Rc;

static SEED_ENV_VAR: &str = "FALSIFY_SEED";

#[derive(Debug, Copy, Clone)]
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

/// Takes the seed value from the `FALSIFY_SEED` environment variable, if set.
pub fn make_test_rng() -> Rc<RefCell<StdRng>> {
    let mut std_rng: StdRng = rand::make_rng();
    let seed: u64 = if let Ok(seed_string) = env::var(SEED_ENV_VAR) {
        if let Ok(seed) = seed_string.parse() {
            seed
        } else {
            panic!("Unable to parse {seed_string:?} as seed!");
        }
    } else {
        std_rng.try_next_u64().unwrap()
    };
    eprintln!("Seed: {seed}");
    let seeded_std_rng = StdRng::seed_from_u64(seed);
    Rc::new(RefCell::new(seeded_std_rng))
}

pub fn make_rng_with_seed(seed: u64) -> Rc<RefCell<StdRng>> {
    eprintln!("Seed: {seed}");
    let seeded_std_rng = StdRng::seed_from_u64(seed);
    Rc::new(RefCell::new(seeded_std_rng))
}

pub fn falsify_with_rejections<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> TestResult + RefUnwindSafe,
    mut arb_t: impl ArbGen<T> + Unpin,
    mut tries: usize,
    reset: impl Fn(),
) -> Option<T> {
    while tries > 0 {
        let value = match pin!(&mut arb_t).resume(()) {
            CoroutineState::Yielded(value) => value,
            CoroutineState::Complete(()) => {
                if tries > 0 {
                    panic!(
                        "Test generator finished prematurely before producing {tries} test cases!"
                    );
                }
                return None;
            }
        };
        let result = catch_unwind(|| test(value.clone()));
        match result {
            Ok(TestResult::Pass) => {
                tries -= 1;
                reset();
                continue;
            }
            Ok(TestResult::Reject) => {
                reset();
                continue;
            }
            Ok(TestResult::Fail) | Err(..) => return Some(value),
        }
    }
    None
}

pub fn falsify_times<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> bool + RefUnwindSafe,
    arb_t: impl ArbGen<T> + Unpin,
    tries: usize,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), arb_t, tries, || {})
}

pub fn falsify<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> bool + RefUnwindSafe,
    arb_t: impl ArbGen<T> + Unpin,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), arb_t, 100, || {})
}

pub fn falsify_with_reset<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> bool + RefUnwindSafe,
    reset: impl Fn(),
    arb_t: impl ArbGen<T> + Unpin,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), arb_t, 100, reset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shrinking::{shrink, shrink_usize_binary_search};
    use std::assert_matches;

    #[test]
    fn test_arb_usize() {
        let rng = make_test_rng();
        let arb = arb_usize(rng);
        assert_matches!(falsify(|n| n % 2 == 0 || n % 2 == 1, arb), None);
    }

    #[test]
    fn test_arb_vec_of_usize() {
        let rng = make_test_rng();
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
    fn test_falsify_then_shrink() {
        let rng = make_test_rng();
        let arb = arb_usize(rng);
        let test = |n| n < 10;
        if let Some(falsifier) = falsify(test, arb) {
            let shrinker = shrink_usize_binary_search(falsifier);
            let smallest = shrink(test, shrinker);
            assert_eq!(smallest, 10);
        };
    }
}
