use std::pin::Pin;
use std::ops::{Coroutine, CoroutineState};

use crate::TestResult;

pub trait ShrinkCoro<Y>: Coroutine<TestResult, Yield = Y, Return = Y> {}
impl<X, Y> ShrinkCoro<Y> for X where X: Coroutine<TestResult, Yield = Y, Return = Y> {}


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

