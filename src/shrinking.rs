use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::{BuildHasher, Hash};
use std::ops::{Coroutine, CoroutineState};
use std::panic::{RefUnwindSafe, catch_unwind};
use std::pin::pin;

use crate::TestResult;

/// Every `Y` type shrinker is `impl ShrinkCoro<Y>`.
pub trait ShrinkCoro<Y>: Coroutine<TestResult, Yield = Y, Return = Y> {}
impl<X, Y> ShrinkCoro<Y> for X where X: Coroutine<TestResult, Yield = Y, Return = Y> {}

pub fn shrink_bool(high: bool) -> impl ShrinkCoro<bool> {
    #[coroutine]
    move |_| {
        if high == false {
            return false;
        }

        match (yield false) {
            TestResult::Fail => false,
            TestResult::Pass | TestResult::Reject => true,
        }
    }
}

// https://doc.rust-lang.org/reference/types/char.html#r-type.char.value
pub fn shrink_char_binary_search(high_char: char) -> impl ShrinkCoro<char> {
    #[coroutine]
    move |_| {
        let mut high: u32 = high_char as u32;

        if high >= 0xE000 {
            let mut low: u32 = 0xE000;

            match (yield char::from_u32(low).unwrap()) {
                TestResult::Fail => {
                    high = low;
                }
                TestResult::Pass | TestResult::Reject => {}
            }

            while low + 1 < high {
                let mid = low + ((high - low) / 2);
                match (yield char::from_u32(mid).unwrap()) {
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
        }

        if high == 0xE000 {
            let mut high = 0xD7FF;
            let mut low: u32 = 0;

            match (yield char::from_u32(low).unwrap()) {
                TestResult::Fail => {
                    high = low;
                }
                TestResult::Pass | TestResult::Reject => {}
            }

            while low + 1 < high {
                let mid = low + ((high - low) / 2);
                match (yield char::from_u32(mid).unwrap()) {
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

            if high < 0xD7FF {
                return char::from_u32(high).unwrap();
            }
        }

        char::from_u32(high).unwrap()
    }
}

macro_rules! shrink_primitive_type_binary_search {
    ($fn:ident, $ty:ty) => {
        pub fn $fn(mut high: $ty) -> impl ShrinkCoro<$ty> {
            #[coroutine]
            move |_| {
                let mut low = <$ty>::MIN;

                match (yield low) {
                    TestResult::Fail => {
                        high = low;
                    }
                    TestResult::Pass | TestResult::Reject => {}
                }

                while low + 1 as $ty < high {
                    let mid = low + ((high - low) / 2 as $ty);
                    match (yield mid) {
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
    };
}

shrink_primitive_type_binary_search!(shrink_u8_binary_search, u8);
shrink_primitive_type_binary_search!(shrink_i8_binary_search, i8);

shrink_primitive_type_binary_search!(shrink_u16_binary_search, u16);
shrink_primitive_type_binary_search!(shrink_i16_binary_search, i16);

shrink_primitive_type_binary_search!(shrink_u32_binary_search, u32);
shrink_primitive_type_binary_search!(shrink_i32_binary_search, i32);
shrink_primitive_type_binary_search!(shrink_f32, f32);

shrink_primitive_type_binary_search!(shrink_u64_binary_search, u64);
shrink_primitive_type_binary_search!(shrink_i64_binary_search, i64);
shrink_primitive_type_binary_search!(shrink_f64, f64);

shrink_primitive_type_binary_search!(shrink_u128_binary_search, u128);
shrink_primitive_type_binary_search!(shrink_i128_binary_search, i128);

shrink_primitive_type_binary_search!(shrink_usize_binary_search, usize);
shrink_primitive_type_binary_search!(shrink_isize_binary_search, isize);

pub fn shrink_usize_exhaustive(falsifier: usize) -> impl ShrinkCoro<usize> {
    #[coroutine]
    move |_| {
        let smallest_falsifier = 'search: {
            for value in 0..=falsifier {
                match (yield value) {
                    TestResult::Fail => break 'search value,
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid = high[0..mid_len].to_vec();
            match (yield mid.clone()) {
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

pub fn shrink_hashset_len_binary_search<T: Eq + Hash + Clone, S: BuildHasher + Clone>(
    mut high: HashSet<T, S>,
) -> impl ShrinkCoro<HashSet<T, S>> {
    #[coroutine]
    move |_| {
        let mut low = HashSet::with_hasher(high.hasher().clone());

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid = {
                let mut mid = HashSet::with_hasher(high.hasher().clone());
                for elem in high.iter().take(mid_len) {
                    mid.insert(elem.clone());
                }
                mid
            };
            match (yield mid.clone()) {
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BTreeSet<T> = high.iter().take(mid_len).cloned().collect();
            match (yield mid.clone()) {
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: VecDeque<T> = high.iter().take(mid_len).cloned().collect();
            match (yield mid.clone()) {
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BinaryHeap<T> = high.iter().take(mid_len).cloned().collect();
            match (yield mid.clone()) {
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: LinkedList<T> = high.iter().take(mid_len).cloned().collect();
            match (yield mid.clone()) {
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

pub fn shrink_hashmap_len_binary_search<K: Eq + Hash + Clone, V: Clone, S: BuildHasher + Clone>(
    mut high: HashMap<K, V, S>,
) -> impl ShrinkCoro<HashMap<K, V, S>> {
    #[coroutine]
    move |_| {
        let mut low = HashMap::with_hasher(high.hasher().clone());

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid = {
                let mut mid = HashMap::with_hasher(high.hasher().clone());
                for (k, v) in high.iter().take(mid_len) {
                    mid.insert(k.clone(), v.clone());
                }
                mid
            };
            match (yield mid.clone()) {
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

        match (yield low.clone()) {
            TestResult::Fail => {
                high = low.clone();
            }
            TestResult::Pass | TestResult::Reject => {}
        }

        while high.len() > low.len() + 1 {
            let mid_len = low.len() + ((high.len() - low.len()) / 2);
            let mid: BTreeMap<K, V> = high
                .iter()
                .take(mid_len)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            match (yield mid.clone()) {
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

/// Tactic: Run test with `None`.  If it failed, we have our smallest falsifier.  If the test
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
            let value = match pin!(&mut shrink_t).resume(test_result) {
                CoroutineState::Yielded(value) => value,
                CoroutineState::Complete(value) => return Some(value),
            };
            test_result = yield Some(value.clone());
        }
    }
}

pub fn shrink_with_rejections<T: Clone + RefUnwindSafe>(
    test: impl Fn(T) -> TestResult + RefUnwindSafe,
    mut shrink_t: impl ShrinkCoro<T> + Unpin,
) -> T {
    let mut result = TestResult::Fail;
    loop {
        let value = match pin!(&mut shrink_t).resume(result) {
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
    shrink_t: impl ShrinkCoro<T> + Unpin,
) -> T {
    shrink_with_rejections(|value| TestResult::from(test(value)), shrink_t)
}

#[cfg(test)]
mod tests {
    use rand::RngExt;

    use crate::{make_rng_with_seed, make_test_rng};

    use crate::sip::HasherBuilder;

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

    #[test]
    fn test_shrink_hashset_binary_search() {
        let rng = make_test_rng();

        // We use multiple `SeedableRandomState`s here and for the test to be reproducible, we need
        // to initialize all of them to the same value (which can be arbitrary).
        let controlled_seed = {
            let mut r = rng.borrow_mut();
            let seed: u64 = r.random();
            seed
        };

        let input = {
            let builder = HasherBuilder::new(make_rng_with_seed(controlled_seed));
            let mut input = HashSet::with_hasher(builder);
            input.insert(1);
            input.insert(2);
            input.insert(3);
            input
        };
        let elem = input.iter().next().unwrap().clone();
        let shrinker = shrink_hashset_len_binary_search(input);
        let smallest_falsifier = shrink(|v| !v.contains(&elem), shrinker);

        let expected = {
            let builder = HasherBuilder::new(make_rng_with_seed(controlled_seed));
            let mut expected = HashSet::with_hasher(builder);
            expected.insert(elem);
            expected
        };
        assert_eq!(smallest_falsifier, expected);
    }

    #[test]
    fn test_shrink_hashmap_binary_search() {
        let rng = make_test_rng();

        // We use multiple `SeedableRandomState`s here and for the test to be reproducible, we need
        // to initialize all of them to the same value (which can be arbitrary).
        let controlled_seed = {
            let mut r = rng.borrow_mut();
            let seed: u64 = r.random();
            seed
        };

        let input = {
            let builder = HasherBuilder::new(make_rng_with_seed(controlled_seed));
            let mut input = HashMap::with_hasher(builder);
            input.insert(1, ());
            input.insert(2, ());
            input.insert(3, ());
            input
        };
        let k = input.iter().next().unwrap().0.clone();
        let shrinker = shrink_hashmap_len_binary_search(input);
        let smallest_falsifier = shrink(|v| !v.contains_key(&k), shrinker);
        let expected = {
            let builder = HasherBuilder::new(make_rng_with_seed(controlled_seed));
            let mut expected = HashMap::with_hasher(builder);
            expected.insert(k, ());
            expected
        };
        assert_eq!(smallest_falsifier, expected);
    }
}
