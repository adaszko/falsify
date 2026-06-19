use rand::{RngExt, rngs::StdRng};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::ops::DerefMut;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::rc::Rc;

pub trait ArbCoro<Y>: Coroutine<Yield = Y, Return = ()> {}
impl<X, Y> ArbCoro<Y> for X where X: Coroutine<Yield = Y, Return = ()> {}

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

pub fn arb_tuple2_of<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T)> {
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

pub fn arb_tuple3_of<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T, T)> {
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

pub fn arb_vec_of<T>(
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

pub fn arb_hashset_of<T: Hash + Eq>(
    mut arb_t: impl ArbCoro<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<HashSet<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut set = HashSet::with_capacity(len);
            for _ in 0..len {
                let t = match Pin::new(&mut arb_t).resume(()) {
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
    mut arb_t: impl ArbCoro<T> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<BTreeSet<T>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut set = BTreeSet::new();
            for _ in 0..len {
                let t = match Pin::new(&mut arb_t).resume(()) {
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
    mut arb_key: impl ArbCoro<K> + Unpin,
    mut arb_val: impl ArbCoro<V> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<HashMap<K, V>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut map = HashMap::with_capacity(len);
            for _ in 0..len {
                let k = match Pin::new(&mut arb_key).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                let v = match Pin::new(&mut arb_val).resume(()) {
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
    mut arb_key: impl ArbCoro<K> + Unpin,
    mut arb_val: impl ArbCoro<V> + Unpin,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<BTreeMap<K, V>> {
    #[coroutine]
    move || {
        loop {
            let len = {
                let mut r = rng.borrow_mut();
                r.random_range(0..max_len)
            };
            let mut map = BTreeMap::new();
            for _ in 0..len {
                let k = match Pin::new(&mut arb_key).resume(()) {
                    CoroutineState::Yielded(t) => t,
                    CoroutineState::Complete(()) => {
                        yield map;
                        return ();
                    }
                };
                let v = match Pin::new(&mut arb_val).resume(()) {
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
    arb_t: Rc<RefCell<dyn ArbCoro<T> + Unpin>>,
    rng: Rc<RefCell<StdRng>>,
    max_len: usize,
) -> impl ArbCoro<Vec<T>> {
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
                    let t = match Pin::new(inner.deref_mut()).resume(()) {
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

pub fn arb_option_of<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Option<T>> {
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

pub fn arb_result_of<T, E>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Result<T, E>> {
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

pub fn arb_box_of<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Box<T>> {
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

pub fn arb_rc_of<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Rc<T>> {
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
