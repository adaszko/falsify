#![cfg(test)]

use super::*;
use rand::RngExt;
use rand::distr::{Alphanumeric, SampleString};
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::cmp::max;
use std::ops::{Coroutine, CoroutineState};
use std::ops::{Deref, DerefMut};
use std::panic::AssertUnwindSafe;
use std::rc::Rc;

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
        let mut arb_vec_coro = arb_vec_of_rc_refcell_of(coro, Rc::clone(&rng), max_width);
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
    arena: Rc<RefCell<Vec<Expr>>>,
    rng: Rc<RefCell<StdRng>>,
    max_width: usize,
    max_depth: usize,
) -> impl ArbCoro<ExprId> + Unpin {
    #[coroutine]
    move || {
        let mut coro_from_depth: Vec<Rc<RefCell<dyn ArbCoro<ExprId> + Unpin>>> = Default::default();
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

fn get_max_tree_width(arena: &[Expr], node_id: ExprId) -> usize {
    match &arena[node_id] {
        Expr::Term { .. } => 0,
        Expr::Opt { child_id } => max(1, get_max_tree_width(arena, *child_id)),
        Expr::Alt { children_ids } => max(
            1,
            children_ids
                .iter()
                .map(|id| get_max_tree_width(arena, *id))
                .max()
                .unwrap_or(0),
        ),
    }
}

fn get_max_tree_depth(arena: &[Expr], node_id: ExprId) -> usize {
    match &arena[node_id] {
        Expr::Term { .. } => 1,
        Expr::Opt { child_id } => 1 + get_max_tree_depth(arena, *child_id),
        Expr::Alt { children_ids } => {
            1 + children_ids
                .iter()
                .map(|id| get_max_tree_depth(arena, *id))
                .max()
                .unwrap_or(1)
        }
    }
}

#[test]
fn test_tree_width_within_limits() {
    let arena = Rc::new(RefCell::new(Vec::new()));
    let rng = make_rng();
    const MAX_WIDTH: usize = 5;
    const MAX_DEPTH: usize = 10;
    let a = AssertUnwindSafe(Rc::clone(&arena));
    let arb = arb_expr(arena, rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify(
        |t| {
            let a = a.borrow();
            get_max_tree_width(a.deref(), t) <= MAX_WIDTH
        },
        arb,
    ) {
        assert!(false, "{}", counterexample);
    }
}

#[test]
fn test_tree_depth_within_limits() {
    let arena = Rc::new(RefCell::new(Vec::new()));
    let rng = make_rng();
    const MAX_WIDTH: usize = 5;
    const MAX_DEPTH: usize = 10;
    let a = AssertUnwindSafe(Rc::clone(&arena));
    let arb = arb_expr(arena, rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify(
        |t| {
            let a = a.borrow();
            get_max_tree_depth(a.deref(), t) <= MAX_DEPTH
        },
        arb,
    ) {
        assert!(false, "{}", counterexample);
    }
}
