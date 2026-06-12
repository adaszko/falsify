use std::ops::{Coroutine, CoroutineState};
use std::panic::{RefUnwindSafe, catch_unwind};
use std::pin::Pin;

use crate::TestResult;

pub trait ShrinkCoro<Y>: Coroutine<TestResult, Yield = Y, Return = Y> {}
impl<X, Y> ShrinkCoro<Y> for X where X: Coroutine<TestResult, Yield = Y, Return = Y> {}

pub fn shrink_with_rejections<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> TestResult + RefUnwindSafe,
    mut root_shrink_coro: impl ShrinkCoro<T> + Unpin,
) -> T {
    let mut result = TestResult::Fail;
    loop {
        let value = match Pin::new(&mut root_shrink_coro).resume(result) {
            CoroutineState::Yielded(value) => value,
            CoroutineState::Complete(falsifier) => return falsifier,
        };
        result = match catch_unwind(|| test(value.clone())) {
            Ok(r) => r,
            Err(..) => TestResult::Fail,
        }
    }
}

pub fn shrink<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> bool + RefUnwindSafe,
    root_shrink_coro: impl ShrinkCoro<T> + Unpin,
) -> T {
    shrink_with_rejections(|value| TestResult::from(test(value)), root_shrink_coro)
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

pub fn shrink_vec_binary_search<T: Clone>(mut high: Vec<T>) -> impl ShrinkCoro<Vec<T>> {
    #[coroutine]
    move |_| {
        let mut low = vec![];
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid = high[0..mid_len].to_vec();
            let res = yield mid.clone();
            match res {
                TestResult::Fail => {
                    high = mid;
                }
                TestResult::Pass | TestResult::Reject => {
                    low = mid;
                }
            }
        }
        high
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_shrink_vec_binary_search() {
        let mut shrinker = shrink_vec_binary_search::<usize>(vec![]);
        let _v: Vec<usize> = vec![];
        assert_matches!(
            Pin::new(&mut shrinker).resume(TestResult::Pass),
            CoroutineState::Complete(_v)
        );
    }
}
