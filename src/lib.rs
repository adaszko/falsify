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

pub trait Arb<Y> {
    fn arb(&self, rng: Rc<RefCell<StdRng>>, size: usize) -> impl ArbCoro<Y>;
}

impl Arb<usize> for Range<usize> {
    fn arb(&self, rng: Rc<RefCell<StdRng>>, num_test_cases: usize) -> impl ArbCoro<usize> {
        #[coroutine]
        move || {
            for _ in 0..num_test_cases {
                let value = {
                    let mut r = rng.borrow_mut();
                    let v = r.random_range(self.clone());
                    v
                };
                yield value;
            }
        }
    }
}

pub fn run<Y>(
    mut root_arb_coro: impl Coroutine<Yield = Y, Return = ()> + Unpin,
    test: impl Fn(Y) -> bool,
) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SysRng;

    #[test]
    fn frob() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = (0..100).arb(rng, 100);
        run(arb, |n| n % 2 == 0 || n % 2 == 1);
    }
}
