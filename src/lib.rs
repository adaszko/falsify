// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait)]

use rand::{RngExt, rngs::StdRng};
use std::cell::RefCell;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::rc::Rc;

pub trait ArbCoro<Y>: Coroutine<Yield = Y, Return = ()> {}
impl<X, Y> ArbCoro<Y> for X where X: Coroutine<Yield = Y, Return = ()> {}

pub trait ShrinkCoro<Y>: Coroutine<TestResult, Yield = Y, Return = Y> {}
impl<X, Y> ShrinkCoro<Y> for X where X: Coroutine<TestResult, Yield = Y, Return = Y> {}

pub fn arb_bool(rng: Rc<RefCell<StdRng>>) -> impl ArbCoro<bool> {
    #[coroutine]
    move || {
        loop {
            let value: bool = {
                let mut r = rng.borrow_mut();
                r.random()
            };
            yield value;
        }
    }
}

pub fn arb_usize(rng: Rc<RefCell<StdRng>>) -> impl ArbCoro<usize> {
    #[coroutine]
    move || {
        loop {
            let value: usize = {
                let mut r = rng.borrow_mut();
                r.random_range(usize::MIN..usize::MAX)
            };
            yield value;
        }
    }
}

pub fn arb_pair<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T)> {
    #[coroutine]
    move || {
        loop {
            let t0 = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t1 = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield (t0, t1);
        }
    }
}

pub fn arb_triple<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T, T)> {
    #[coroutine]
    move || {
        loop {
            let t0 = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t1 = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t2 = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield (t0, t1, t2);
        }
    }
}

pub fn arb_vec<T>(
    mut arb_t: impl ArbCoro<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<Vec<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let t = match Pin::new(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => return (),
                };
                v.push(t);
            }
            yield v;
        }
    }
}

pub fn arb_option<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Option<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Some(t);
        }
    }
}

pub fn arb_result<T, E>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Result<T, E>> {
    #[coroutine]
    move || {
        loop {
            let t = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Ok(t);
        }
    }
}

pub fn arb_box<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Box<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Box::new(t);
        }
    }
}

pub fn arb_rc<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Rc<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match Pin::new(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Rc::new(t);
        }
    }
}

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

pub fn shrink_usize_binary_search(mut high: usize) -> impl ShrinkCoro<usize> {
    #[coroutine]
    move |_| {
        let mut low = 0;
        while high > low + 1 {
            let mid = low + ((high - low) / 2);
            let res = yield mid;
            match res {
                TestResult::Fail => {
                    // test failed after previously failing -- narrow down the range further
                    high = mid;
                }
                TestResult::Pass | TestResult::Reject => {
                    // test succeeded after previously failing
                    low = mid;
                }
            }
        }
        high
    }
}

pub fn shrink_usize_exhaustive(falsifier: usize) -> impl ShrinkCoro<usize> {
    #[coroutine]
    move |_| {
        let smallest_falsifier = 'search: {
            for val in 0..=falsifier {
                let res = yield val;
                match res {
                    TestResult::Fail => break 'search val,
                    TestResult::Pass | TestResult::Reject => continue,
                }
            }
            falsifier
        };
        smallest_falsifier
    }
}

pub fn shrink<T: Clone>(
    mut root_shrink_coro: impl ShrinkCoro<T> + Unpin,
    test: impl Fn(T) -> TestResult,
) -> T {
    let mut res = TestResult::Fail;
    loop {
        let value = match Pin::new(&mut root_shrink_coro).resume(res) {
            CoroutineState::Yielded(value) => value,
            CoroutineState::Complete(falsifier) => return falsifier,
        };
        res = test(value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
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
