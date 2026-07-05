use rand::{RngExt, rngs::StdRng};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;
use std::ops::DerefMut;
use std::ops::{Coroutine, CoroutineState};
use std::pin::pin;
use std::rc::Rc;

/// Every generator of arbitrary test inputs of type `Y` is `impl ArbGen<Y>`.
pub trait ArbGen<Y>: Coroutine<Yield = Y, Return = ()> {}
impl<X, Y> ArbGen<Y> for X where X: Coroutine<Yield = Y, Return = ()> {}

macro_rules! arb_primitive_type {
    ($fn:ident, $ty:ty) => {
        pub fn $fn(rng: Rc<RefCell<StdRng>>) -> impl ArbGen<$ty> {
            #[coroutine]
            move || {
                loop {
                    let value: $ty = {
                        let mut r = rng.borrow_mut();
                        r.random()
                    };
                    yield value;
                }
            }
        }
    };
}

arb_primitive_type!(arb_u8, u8);
arb_primitive_type!(arb_i8, i8);

arb_primitive_type!(arb_bool, bool);

arb_primitive_type!(arb_u16, u16);
arb_primitive_type!(arb_i16, i16);

arb_primitive_type!(arb_u32, u32);
arb_primitive_type!(arb_i32, i32);
arb_primitive_type!(arb_f32, f32);

arb_primitive_type!(arb_u64, u64);
arb_primitive_type!(arb_i64, i64);
arb_primitive_type!(arb_f64, f64);

arb_primitive_type!(arb_u128, u128);
arb_primitive_type!(arb_i128, i128);

arb_primitive_type!(arb_char, char);

pub fn arb_isize(rng: Rc<RefCell<StdRng>>) -> impl ArbGen<isize> {
    cfg_select! {
        target_pointer_width = "64" => {
            #[coroutine]
            move || {
                loop {
                    let value: i64 = {
                        let mut r = rng.borrow_mut();
                        r.random()
                    };
                    yield value as isize;
                }
            }
        }
        target_pointer_width = "32" => {
            #[coroutine]
            move || {
                loop {
                    let value: i32 = {
                        let mut r = rng.borrow_mut();
                        r.random()
                    };
                    yield value as isize;
                }
            }
        }
        target_pointer_width = "16" => {
            #[coroutine]
            move || {
                loop {
                    let value: i16 = {
                        let mut r = rng.borrow_mut();
                        r.random()
                    };
                    yield value as isize;
                }
            }
        }
        _ => {
            compile_error!("Unsupported pointer width!");
        }
    }
}

pub fn arb_usize(rng: Rc<RefCell<StdRng>>) -> impl ArbGen<usize> {
    #[coroutine]
    move || {
        loop {
            let value: usize = {
                let mut r = rng.borrow_mut();
                r.random_range(usize::MIN..=usize::MAX)
            };
            yield value;
        }
    }
}

pub fn arb_tuple2_of<T>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<(T, T)> {
    #[coroutine]
    move || {
        loop {
            let t0 = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t1 = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield (t0, t1);
        }
    }
}

pub fn arb_tuple3_of<T>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<(T, T, T)> {
    #[coroutine]
    move || {
        loop {
            let t0 = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t1 = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            let t2 = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield (t0, t1, t2);
        }
    }
}

pub fn arb_vec_of<T>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<Vec<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield v;
                        return ();
                    }
                };
                v.push(t);
            }
            yield v;
        }
    }
}

pub fn arb_vec_deque_of<T>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<VecDeque<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut q = VecDeque::with_capacity(len);
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield q;
                        return ();
                    }
                };
                let direction: bool = {
                    let mut r = rng.borrow_mut();
                    r.random()
                };
                if direction {
                    q.push_back(t);
                } else {
                    q.push_front(t);
                }
            }
            yield q;
        }
    }
}

pub fn arb_binary_heap_of<T: Ord>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<BinaryHeap<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut h = BinaryHeap::with_capacity(len);
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield h;
                        return ();
                    }
                };
                h.push(t);
            }
            yield h;
        }
    }
}

pub fn arb_linked_list_of<T: Ord>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<LinkedList<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut l = LinkedList::new();
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield l;
                        return ();
                    }
                };
                let direction: bool = {
                    let mut r = rng.borrow_mut();
                    r.random()
                };
                if direction {
                    l.push_back(t);
                } else {
                    l.push_front(t);
                }
            }
            yield l;
        }
    }
}

pub fn arb_hashset_of<T: Hash + Eq>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<HashSet<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut set = HashSet::with_capacity(len);
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield set;
                        return ();
                    }
                };
                set.insert(t);
            }
            yield set;
        }
    }
}

pub fn arb_btreeset_of<T: Ord>(
    mut arb_t: impl ArbGen<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<BTreeSet<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut set = BTreeSet::new();
            for _ in 0..len {
                let t = match pin!(&mut arb_t).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield set;
                        return ();
                    }
                };
                set.insert(t);
            }
            yield set;
        }
    }
}

pub fn arb_hashmap_of<K: Eq + Hash, V>(
    mut arb_key: impl ArbGen<K> + Unpin,
    mut arb_val: impl ArbGen<V> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<HashMap<K, V>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut map = HashMap::with_capacity(len);
            for _ in 0..len {
                let k = match pin!(&mut arb_key).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                let v = match pin!(&mut arb_val).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                map.insert(k, v);
            }
            yield map;
        }
    }
}

pub fn arb_btreemap_of<K: Ord, V>(
    mut arb_key: impl ArbGen<K> + Unpin,
    mut arb_val: impl ArbGen<V> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<BTreeMap<K, V>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut map = BTreeMap::new();
            for _ in 0..len {
                let k = match pin!(&mut arb_key).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                let v = match pin!(&mut arb_val).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                map.insert(k, v);
            }
            yield map;
        }
    }
}

pub fn arb_vec_of_rc_refcell_of<T>(
    arb_t: Rc<RefCell<dyn ArbGen<T> + Unpin>>,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbGen<Vec<T>> {
    #[coroutine]
    move || {
        loop {
            let v = {
                let mut inner = arb_t.borrow_mut();
                let len = {
                    let mut r = rng.borrow_mut();
                    r.random_range(0..max_len)
                };
                let mut v = Vec::with_capacity(len);
                for _ in 0..len {
                    let t = match pin!(inner.deref_mut()).resume(()) {
                        CoroutineState::Yielded(t) => t,
                        CoroutineState::Complete(()) => return (),
                    };
                    v.push(t);
                }
                v
            };
            yield v;
        }
    }
}

pub fn arb_option_of<T>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<Option<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Some(t);
        }
    }
}

pub fn arb_result_of<T, E>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<Result<T, E>> {
    #[coroutine]
    move || {
        loop {
            let t = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Ok(t);
        }
    }
}

pub fn arb_box_of<T>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<Box<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Box::new(t);
        }
    }
}

pub fn arb_rc_of<T>(mut arb_t: impl ArbGen<T> + Unpin) -> impl ArbGen<Rc<T>> {
    #[coroutine]
    move || {
        loop {
            let t = match pin!(&mut arb_t).resume(()) {
                CoroutineState::Yielded(t) => t,
                CoroutineState::Complete(()) => return (),
            };
            yield Rc::new(t);
        }
    }
}
