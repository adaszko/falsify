use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;
use std::ops::{Coroutine, CoroutineState};
use std::panic::{RefUnwindSafe, catch_unwind};
use std::pin::Pin;

use crate::TestResult;

pub trait ShrinkCoro<Y>: Coroutine<TestResult, Yield = Y, Return = Y> {}
impl<X, Y> ShrinkCoro<Y> for X where X: Coroutine<TestResult, Yield = Y, Return = Y> {}

pub fn shrink_usize_binary_search(mut high: usize) -> impl ShrinkCoro<usize> {
    #[coroutine]
    move |_| {
        let mut low = 0;
        while high > low + 1 {
            let mid = low + ((high - low) / 2);
            let test_result = yield mid;
            match test_result {
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
                let test_result = yield val;
                match test_result {
                    TestResult::Fail => break 'search val,
                    TestResult::Pass | TestResult::Reject => continue,
                }
            }
            falsifier
        };
        smallest_falsifier
    }
}

pub fn shrink_vec_len_binary_search<T: Clone>(mut high: Vec<T>) -> impl ShrinkCoro<Vec<T>> {
    #[coroutine]
    move |_| {
        let mut low = vec![];
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid = high[0..mid_len].to_vec();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_hashset_len_binary_search<T: Eq + Hash + Clone>(
    mut high: HashSet<T>,
) -> impl ShrinkCoro<HashSet<T>> {
    #[coroutine]
    move |_| {
        let mut low = HashSet::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: HashSet<T> = high.iter().take(mid_len).cloned().collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_btreeset_len_binary_search<T: Ord + Clone>(
    mut high: BTreeSet<T>,
) -> impl ShrinkCoro<BTreeSet<T>> {
    #[coroutine]
    move |_| {
        let mut low = BTreeSet::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BTreeSet<T> = high.iter().take(mid_len).cloned().collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_vec_deque_len_binary_search<T: Ord + Clone>(
    mut high: VecDeque<T>,
) -> impl ShrinkCoro<VecDeque<T>> {
    #[coroutine]
    move |_| {
        let mut low = VecDeque::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: VecDeque<T> = high.iter().take(mid_len).cloned().collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_binary_heap_len_binary_search<T: Ord + Clone>(
    mut high: BinaryHeap<T>,
) -> impl ShrinkCoro<BinaryHeap<T>> {
    #[coroutine]
    move |_| {
        let mut low = BinaryHeap::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BinaryHeap<T> = high.iter().take(mid_len).cloned().collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_linked_list_len_binary_search<T: Ord + Clone>(
    mut high: LinkedList<T>,
) -> impl ShrinkCoro<LinkedList<T>> {
    #[coroutine]
    move |_| {
        let mut low = LinkedList::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: LinkedList<T> = high.iter().take(mid_len).cloned().collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_hashmap_len_binary_search<K: Eq + Hash + Clone, V: Clone>(
    mut high: HashMap<K, V>,
) -> impl ShrinkCoro<HashMap<K, V>> {
    #[coroutine]
    move |_| {
        let mut low = HashMap::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: HashMap<K, V> = high
                .iter()
                .take(mid_len)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let test_result = yield mid.clone();
            match test_result {
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

pub fn shrink_btreemap_len_binary_search<K: Ord + Clone, V: Clone>(
    mut high: BTreeMap<K, V>,
) -> impl ShrinkCoro<BTreeMap<K, V>> {
    #[coroutine]
    move |_| {
        let mut low = BTreeMap::new();
        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BTreeMap<K, V> = high
                .iter()
                .take(mid_len)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let test_result = yield mid.clone();
            match test_result {
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

/// Strategy: Run test with `None`.  If it failed, we have our smallest falsifier.  If the test
/// succeeded, produce successive `Some(t)`, where `t` comes from the underlying shirinker for `t`.
pub fn shrink_option<T: Clone>(
    mut shrink_t: impl ShrinkCoro<T> + Unpin,
) -> impl ShrinkCoro<Option<T>> {
    #[coroutine]
    move |_| {
        let test_result = yield None;
        match test_result {
            TestResult::Fail => {
                return None;
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        let mut test_result = TestResult::Fail;
        loop {
            let value = match Pin::new(&mut shrink_t).resume(test_result) {
                CoroutineState::Yielded(value) => value,
                CoroutineState::Complete(value) => return Some(value),
            };
            test_result = yield Some(value.clone());
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shrink_usize_binary_search() {
        let shrinker = shrink_usize_binary_search(123);
        let smallest_falsifier = shrink(|v| v < 10, shrinker);
        assert_eq!(smallest_falsifier, 10);
    }

    #[test]
    fn test_shrink_usize_exhaustive() {
        let shrinker = shrink_usize_exhaustive(123);
        let smallest_falsifier = shrink(|v| v < 10, shrinker);
        assert_eq!(smallest_falsifier, 10);
    }

    #[test]
    fn test_shrink_vec_binary_search() {
        let shrinker = shrink_vec_len_binary_search(vec![1, 2, 3]);
        let smallest_falsifier = shrink(|v| !v.contains(&1), shrinker);
        assert_eq!(smallest_falsifier, &[1]);
    }

    #[test]
    fn test_shrink_btreeset_binary_search() {
        let shrinker = shrink_btreeset_len_binary_search(BTreeSet::from_iter(&[1, 2, 3]));
        let smallest_falsifier = shrink(|v| !v.contains(&1), shrinker);
        assert_eq!(smallest_falsifier, BTreeSet::from_iter(&[1]));
    }

    #[test]
    fn test_shrink_vec_deque_binary_search() {
        let shrinker = shrink_vec_deque_len_binary_search(VecDeque::from_iter([1, 2, 3]));
        let smallest_falsifier = shrink(|v| !v.contains(&1), shrinker);
        assert_eq!(smallest_falsifier, VecDeque::from_iter([1]));
    }

    #[test]
    fn test_shrink_linked_list_binary_search() {
        let shrinker = shrink_linked_list_len_binary_search(LinkedList::from_iter([1, 2, 3]));
        let smallest_falsifier = shrink(|v| !v.contains(&1), shrinker);
        assert_eq!(smallest_falsifier, LinkedList::from_iter([1]));
    }

    #[test]
    fn test_shrink_btreemap_binary_search() {
        let shrinker =
            shrink_btreemap_len_binary_search(BTreeMap::from([(1, ()), (2, ()), (3, ())]));
        let smallest_falsifier = shrink(|v| !v.contains_key(&1), shrinker);
        assert_eq!(smallest_falsifier, BTreeMap::from([(1, ())]));
    }
}
