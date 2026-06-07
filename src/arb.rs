use rand::{RngExt, rngs::StdRng};
use std::cell::RefCell;
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

pub fn arb_tuple2<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T)> {
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

pub fn arb_tuple3<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T, T)> {
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
