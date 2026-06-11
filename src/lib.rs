// https://doc.rust-lang.org/unstable-book/language-features/coroutines.html
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

mod arb;
mod shrink;

pub use arb::*;
pub use shrink::*;

use std::ops::CoroutineState;
use std::pin::Pin;

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

pub fn falsify_with_rejections<T: Clone>(
    test: impl Fn(T) -> TestResult,
    mut root_arb_coro: impl ArbCoro<T> + Unpin,
    mut tries: usize,
) -> Option<T> {
    while tries > 0 {
        // TODO Wrap test in catch_unwind()
        match Pin::new(&mut root_arb_coro).resume(()) {
            CoroutineState::Yielded(val) => match test(val.clone()) {
                TestResult::Pass => {
                    tries -= 1;
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

pub fn falsify_with_tries<T: Clone>(
    test: impl Fn(T) -> bool,
    root_arb_coro: impl ArbCoro<T> + Unpin,
    tries: usize,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), root_arb_coro, tries)
}

pub fn falsify<T: Clone>(
    test: impl Fn(T) -> bool,
    root_arb_coro: impl ArbCoro<T> + Unpin,
) -> Option<T> {
    falsify_with_rejections(|value| TestResult::from(test(value)), root_arb_coro, 100)
}

#[cfg(test)]
mod tests {
    use crate::shrink::{shrink, shrink_usize_exhaustive};

    use super::*;
    use rand::distr::{Alphanumeric, SampleString};
    use rand::rngs::{StdRng, SysRng};
    use rand::{RngExt, SeedableRng};
    use std::assert_matches;
    use std::cell::RefCell;
    use std::ops::DerefMut;
    use std::ops::{Coroutine, CoroutineState};
    use std::rc::Rc;

    #[test]
    fn test_arb_usize() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        assert_matches!(falsify(|n| n % 2 == 0 || n % 2 == 1, arb), None);
    }

    #[test]
    fn test_arb_vec_usize() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb_usize = arb_usize(rng.clone());
        let arb_vec = arb_vec(arb_usize, rng, 50);
        assert_matches!(
            falsify(
                |mut v| {
                    let original = v.clone();
                    v.reverse();
                    v.reverse();
                    v == original
                },
                arb_vec,
            ),
            None
        );
    }

    #[test]
    fn test_shrink() {
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_usize(rng);
        let test = |n| n % 2 == 0;
        if let Some(falsifier) = falsify(test, arb) {
            let shrink_strategy = shrink_usize_exhaustive(falsifier);
            shrink(test, shrink_strategy);
        };
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ExprId(pub usize);

    impl std::fmt::Display for ExprId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::ops::Index<ExprId> for Vec<Expr> {
        type Output = Expr;

        fn index(&self, index: ExprId) -> &Self::Output {
            &self[index.0]
        }
    }

    impl std::ops::Index<ExprId> for [Expr] {
        type Output = Expr;

        fn index(&self, index: ExprId) -> &Self::Output {
            &self[index.0]
        }
    }

    fn alloc(arena: Rc<RefCell<Vec<Expr>>>, expr: Expr) -> ExprId {
        let mut a = arena.borrow_mut();
        let id = a.len();
        a.push(expr);
        ExprId(id)
    }

    // Sample arena-based tree structure for testing
    #[derive(Clone)]
    pub enum Expr {
        Term { term: String },
        Opt { child_id: ExprId },
        Alt { children_ids: Vec<ExprId> },
    }

    fn arb_term(arena: Rc<RefCell<Vec<Expr>>>, rng: Rc<RefCell<StdRng>>) -> impl ArbCoro<ExprId> {
        #[coroutine]
        move || {
            loop {
                let term: String = {
                    let mut r = rng.borrow_mut();
                    Alphanumeric.sample_string(&mut r, 16)
                };
                let expr = Expr::Term { term };
                let expr_id = alloc(Rc::clone(&arena), expr);
                yield expr_id;
            }
        }
    }

    fn arb_opt(
        arena: Rc<RefCell<Vec<Expr>>>,
        coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>>>>,
        remaining_depth: usize,
    ) -> impl ArbCoro<ExprId> {
        #[coroutine]
        move || {
            loop {
                let child_id = {
                    let mut coro = coro_from_depth[remaining_depth - 1].borrow_mut();
                    match Pin::new(coro.deref_mut()).resume(()) {
                        CoroutineState::Yielded(child_id) => child_id,
                        CoroutineState::Complete(()) => return (),
                    }
                };
                let expr = Expr::Opt { child_id };
                let expr_id = alloc(Rc::clone(&arena), expr);
                yield expr_id;
            }
        }
    }

    fn arb_alt(
        arena: Rc<RefCell<Vec<Expr>>>,
        rng: Rc<RefCell<StdRng>>,
        max_width: usize,
        coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>>>>,
        remaining_depth: usize,
    ) -> impl ArbCoro<ExprId> {
        #[coroutine]
        move || {
            let coro = Rc::clone(&coro_from_depth[remaining_depth - 1]);
            let mut arb_vec_coro = arb_vec_rc_refcell(coro, Rc::clone(&rng), max_width);
            loop {
                let children_ids = match Pin::new(&mut arb_vec_coro).resume(()) {
                    CoroutineState::Yielded(subexpr) => subexpr,
                    CoroutineState::Complete(()) => return (),
                };

                let expr = Expr::Alt { children_ids };
                let expr_id = alloc(Rc::clone(&arena), expr);
                yield expr_id;
            }
        }
    }

    fn do_arb_expr(
        arena: Rc<RefCell<Vec<Expr>>>,
        rng: Rc<RefCell<StdRng>>,
        max_width: usize,
        coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>>>>,
        remaining_depth: usize,
    ) -> Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>> {
        let coro = #[coroutine]
        move || {
            let mut term = arb_term(Rc::clone(&arena), Rc::clone(&rng));

            if remaining_depth == 1 {
                loop {
                    let expr_id = match Pin::new(&mut term).resume(()) {
                        CoroutineState::Yielded(child_id) => child_id,
                        CoroutineState::Complete(()) => return (),
                    };
                    yield expr_id;
                }
            }

            let mut opt = arb_opt(
                Rc::clone(&arena),
                Rc::clone(&coro_from_depth),
                remaining_depth - 1,
            );
            let mut alt = arb_alt(
                Rc::clone(&arena),
                Rc::clone(&rng),
                max_width,
                Rc::clone(&coro_from_depth),
                remaining_depth - 1,
            );

            loop {
                let variant_index = {
                    let mut r = rng.borrow_mut();
                    r.random_range(0..=2)
                };
                {
                    let mut a = arena.borrow_mut();
                    a.clear();
                };
                let expr_id = match variant_index {
                    0 => match Pin::new(&mut term).resume(()) {
                        CoroutineState::Yielded(child_id) => child_id,
                        CoroutineState::Complete(()) => return (),
                    },
                    1 => match Pin::new(&mut opt).resume(()) {
                        CoroutineState::Yielded(child_id) => child_id,
                        CoroutineState::Complete(()) => return (),
                    },
                    2 => match Pin::new(&mut alt).resume(()) {
                        CoroutineState::Yielded(child_id) => child_id,
                        CoroutineState::Complete(()) => return (),
                    },
                    _ => unreachable!(),
                };
                yield expr_id;
            }
        };
        Rc::new(RefCell::new(coro))
    }

    fn arb_expr(
        arena: Rc<RefCell<Vec<Expr>>>,
        rng: Rc<RefCell<StdRng>>,
        max_width: usize,
        max_depth: usize,
    ) -> impl ArbCoro<ExprId> + Unpin {
        #[coroutine]
        move || {
            let mut coro_from_depth: Vec<Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>>> =
                Default::default();
            for i in 0..max_depth {
                let coro = do_arb_expr(
                    Rc::clone(&arena),
                    Rc::clone(&rng),
                    max_width,
                    Rc::new(coro_from_depth[0..i].to_owned()),
                    i,
                );
                coro_from_depth.push(coro);
            }
            loop {
                let expr_id = {
                    let mut expr = coro_from_depth[max_depth - 1].borrow_mut();
                    let expr_id = match Pin::new(expr.deref_mut()).resume(()) {
                        CoroutineState::Yielded(expr_id) => expr_id,
                        CoroutineState::Complete(()) => return (),
                    };
                    expr_id
                };
                yield expr_id;
            }
        }
    }

    #[test]
    fn test_tree_within_limits() {
        let arena = Rc::new(RefCell::new(Vec::new()));
        let rng = Rc::new(RefCell::new(StdRng::try_from_rng(&mut SysRng).unwrap()));
        let arb = arb_expr(arena, rng, 5, 10);
        if let Some(counterexample) = falsify(
            |t| {
                dbg!(t);
                true
            },
            arb,
        ) {
            assert!(false, "{}", counterexample);
        }
    }
}
