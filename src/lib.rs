// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait)]

use rand::{RngExt, rngs::StdRng};
use std::cell::RefCell;
use std::ops::Range;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::rc::Rc;

pub trait ArbCoro<Y>: Coroutine<Yield = Y, Return = ()> {}
impl<X, Y> ArbCoro<Y> for X where X: Coroutine<Yield = Y, Return = ()> {}

pub fn arb_bool() -> impl ArbCoro<bool> {
    #[coroutine]
    move || {
        yield false;
        yield true;
    }
}

pub fn arb_range_usize(
    range: Range<usize>,
    rng: Rc<RefCell<StdRng>>,
    num_tests: usize,
) -> impl ArbCoro<usize> {
    #[coroutine]
    move || {
        for _ in 0..num_tests {
            let value = {
                let mut r = rng.borrow_mut();
                let v = r.random_range(range.clone());
                v
            };
            yield value;
        }
    }
}

pub fn arb_pair<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T)> {
    #[coroutine]
    move || {
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

pub fn arb_triple<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<(T, T, T)> {
    #[coroutine]
    move || {
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

pub fn arb_vec<T>(mut arb_t: impl ArbCoro<T> + Unpin, len: usize) -> impl ArbCoro<Vec<T>> {
    #[coroutine]
    move || {
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

pub fn arb_box<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Box<T>> {
    #[coroutine]
    move || {
        let t = match Pin::new(&mut arb_t).resume(()) {
            CoroutineState::Yielded(t) => t,
            CoroutineState::Complete(()) => return (),
        };
        yield Box::new(t);
    }
}

pub fn arb_rc<T>(mut arb_t: impl ArbCoro<T> + Unpin) -> impl ArbCoro<Rc<T>> {
    #[coroutine]
    move || {
        let t = match Pin::new(&mut arb_t).resume(()) {
            CoroutineState::Yielded(t) => t,
            CoroutineState::Complete(()) => return (),
        };
        yield Rc::new(t);
    }
}

pub fn run<Y>(mut root_arb_coro: impl ArbCoro<Y> + Unpin, test: impl Fn(Y) -> bool) -> bool {
    loop {
        match Pin::new(&mut root_arb_coro).resume(()) {
            CoroutineState::Yielded(val) => {
                if !test(val) {
                    return false;
                }
            }
            CoroutineState::Complete(()) => return true,
        }
    }
}

pub trait Arb<Y> {
    fn arb(&self, rng: Rc<RefCell<StdRng>>, size: usize) -> impl ArbCoro<Y>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SysRng;

    #[test]
    fn frob() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_range_usize(0..100, rng, 100);
        run(arb, |n| n % 2 == 0 || n % 2 == 1);
    }

    #[test]
    fn frob_vec() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb_range_usize = arb_range_usize(0..100, rng, 100);
        let arb_vec = arb_vec(arb_range_usize, 50);
        run(arb_vec, |mut v| {
            let original = v.clone();
            v.reverse();
            v.reverse();
            v == original
        });
    }
}
