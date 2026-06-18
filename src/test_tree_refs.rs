#![cfg(test)]

use super::*;
use rand::RngExt;
use rand::distr::{Alphanumeric, SampleString};
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::cmp::max;
use std::ops::DerefMut;
use std::ops::{Coroutine, CoroutineState};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Expr {
    Term { term: String },
    Opt { child: Rc<Expr> },
    Alt { children: Vec<Rc<Expr>> },
}

fn arb_term(rng: Rc<RefCell<StdRng>>) -> impl ArbCoro<Rc<Expr>> {
    #[coroutine]
    move || {
        loop {
            let term: String = {
                let mut r = rng.borrow_mut();
                Alphanumeric.sample_string(&mut r, 16)
            };
            let expr = Expr::Term { term };
            yield Rc::new(expr);
        }
    }
}

fn arb_opt(
    coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<Rc<Expr>> + Unpin>>>>,
    remaining_depth: usize,
) -> impl ArbCoro<Rc<Expr>> {
    #[coroutine]
    move || {
        loop {
            let child = {
                let mut coro = coro_from_depth[remaining_depth - 1].borrow_mut();
                match Pin::new(coro.deref_mut()).resume(()) {
                    CoroutineState::Yielded(child_id) => child_id,
                    CoroutineState::Complete(()) => return (),
                }
            };
            let expr = Expr::Opt { child: child };
            yield Rc::new(expr);
        }
    }
}

fn arb_alt(
    rng: Rc<RefCell<StdRng>>,
    max_width: usize,
    coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<Rc<Expr>> + Unpin>>>>,
    remaining_depth: usize,
) -> impl ArbCoro<Rc<Expr>> {
    #[coroutine]
    move || {
        let coro = Rc::clone(&coro_from_depth[remaining_depth - 1]);
        let mut arb_vec_coro = arb_vec_of_rc_refcell_of(coro, Rc::clone(&rng), max_width);
        loop {
            let children = match Pin::new(&mut arb_vec_coro).resume(()) {
                CoroutineState::Yielded(subexpr) => subexpr,
                CoroutineState::Complete(()) => return (),
            };

            let expr = Expr::Alt { children: children };
            yield Rc::new(expr);
        }
    }
}

fn do_arb_expr(
    rng: Rc<RefCell<StdRng>>,
    max_width: usize,
    coro_from_depth: Rc<Vec<Rc<RefCell<dyn ArbCoro<Rc<Expr>> + Unpin>>>>,
    remaining_depth: usize,
) -> Rc<RefCell<dyn ArbCoro<Rc<Expr>> + Unpin>> {
    let coro = #[coroutine]
    move || {
        let mut term = arb_term(Rc::clone(&rng));

        if remaining_depth == 1 {
            loop {
                let expr_id = match Pin::new(&mut term).resume(()) {
                    CoroutineState::Yielded(child_id) => child_id,
                    CoroutineState::Complete(()) => return (),
                };
                yield expr_id;
            }
        }

        let mut opt = arb_opt(Rc::clone(&coro_from_depth), remaining_depth - 1);
        let mut alt = arb_alt(
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
            let expr_id = match variant_index {
                0 => match Pin::new(&mut term).resume(()) {
                    CoroutineState::Yielded(child_id) => child_id,
                    CoroutineState::Complete(()) => return (), // TODO fall back on other variants
                },
                1 => match Pin::new(&mut opt).resume(()) {
                    CoroutineState::Yielded(child_id) => child_id,
                    CoroutineState::Complete(()) => return (), // TODO fall back on other variants
                },
                2 => match Pin::new(&mut alt).resume(()) {
                    CoroutineState::Yielded(child_id) => child_id,
                    CoroutineState::Complete(()) => return (), // TODO fall back on other variants
                },
                _ => unreachable!(),
            };
            yield expr_id;
        }
    };
    Rc::new(RefCell::new(coro))
}

fn arb_expr(
    rng: Rc<RefCell<StdRng>>,
    max_width: usize,
    max_depth: usize,
) -> impl ArbCoro<Rc<Expr>> + Unpin {
    #[coroutine]
    move || {
        let mut coro_from_depth: Vec<Rc<RefCell<dyn ArbCoro<Rc<Expr>> + Unpin>>> =
            Default::default();
        for i in 0..max_depth {
            let coro = do_arb_expr(
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

fn get_max_tree_width(node: Rc<Expr>) -> usize {
    match node.as_ref() {
        Expr::Term { .. } => 0,
        Expr::Opt { child } => max(1, get_max_tree_width(Rc::clone(&child))),
        Expr::Alt { children } => max(
            1,
            children
                .iter()
                .map(|child| get_max_tree_width(Rc::clone(&child)))
                .max()
                .unwrap_or(0),
        ),
    }
}

fn get_max_tree_depth(node_id: Rc<Expr>) -> usize {
    match node_id.as_ref() {
        Expr::Term { .. } => 1,
        Expr::Opt { child } => 1 + get_max_tree_depth(Rc::clone(&child)),
        Expr::Alt { children } => {
            1 + children
                .iter()
                .map(|child| get_max_tree_depth(Rc::clone(&child)))
                .max()
                .unwrap_or(1)
        }
    }
}

#[test]
fn test_tree_width_within_limits() {
    let rng = make_rng();
    const MAX_WIDTH: usize = 5;
    const MAX_DEPTH: usize = 10;
    let arb = arb_expr(rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify(|t| get_max_tree_width(t) <= MAX_WIDTH, arb) {
        assert!(false, "{:?}", counterexample);
    }
}

#[test]
fn test_tree_depth_within_limits() {
    let rng = make_rng();
    const MAX_WIDTH: usize = 5;
    const MAX_DEPTH: usize = 10;
    let arb = arb_expr(rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify(|t| get_max_tree_depth(t) <= MAX_DEPTH, arb) {
        assert!(false, "{:?}", counterexample);
    }
}
