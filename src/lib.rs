// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

use rand::{
    SeedableRng,
    rngs::{StdRng, SysRng},
};

pub struct TestDriver {
    rng: StdRng,
}

impl TestDriver {
    fn new() -> Self {
        let rng = StdRng::try_from_rng(&mut SysRng).unwrap();
        Self { rng }
    }

    pub fn from_seed(seed: u64) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        Self { rng }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::{Coroutine, CoroutineState};
    use std::pin::Pin;

    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    fn reverse_round_trip() {
        // TODO: Generate 100s of test cases, not just one
        let mut driver = TestDriver::new();
        let mut nums: Vec<i32> = (1..100).collect();
        nums.shuffle(&mut driver.rng);
        let before = nums.clone();
        nums.reverse();
        nums.reverse();
        assert_eq!(nums, before);
    }

    #[test]
    fn it_works() {
        let mut coroutine = #[coroutine]
        || {
            yield 1;
            return "foo";
        };

        match Pin::new(&mut coroutine).resume(()) {
            CoroutineState::Yielded(1) => {}
            _ => panic!("unexpected value from resume"),
        }
        match Pin::new(&mut coroutine).resume(()) {
            CoroutineState::Complete("foo") => {}
            _ => panic!("unexpected value from resume"),
        }
    }
}
