#![cfg(test)]

use super::*;
use rand::distr::{Alphanumeric, SampleString};
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use std::cell::RefCell;
use std::cmp::max;
use std::ops::DerefMut;
use std::ops::{Coroutine, CoroutineState};
use std::panic::AssertUnwindSafe;
use std::rc::Rc;

// Sample arena-based tree structure for testing
#[derive(Debug, Clone)]
pub enum Expr {
    Term { term: String },
    Opt { child_id: ExprId },
    Alt { children_ids: Vec<ExprId> },
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

fn arb_term(arena: Rc<RefCell<Vec<Expr>>>, rng: Rc<RefCell<StdRng>>) -> impl ArbGen<ExprId> {
    #[coroutine]
    move || loop {
        let term: String = {
            let mut r = rng.borrow_mut();
            Alphanumeric.sample_string(&mut r, 16)
        };
        let expr = Expr::Term { term };
        let expr_id = alloc(Rc::clone(&arena), expr);
        yield expr_id;
    }
}

fn arb_opt(
    arena: Rc<RefCell<Vec<Expr>>>,
    child_coro: Rc<RefCell<dyn ArbGen<ExprId> + Unpin>>,
) -> impl ArbGen<ExprId> {
    #[coroutine]
    move || loop {
        let child_id = {
            let mut coro = child_coro.borrow_mut();
            match pin!(coro.deref_mut()).resume(()) {
                CoroutineState::Yielded(child_id) => child_id,
                CoroutineState::Complete(()) => return (),
            }
        };
        let expr = Expr::Opt { child_id };
        let expr_id = alloc(Rc::clone(&arena), expr);
        yield expr_id;
    }
}

fn arb_alt(
    arena: Rc<RefCell<Vec<Expr>>>,
    rng: Rc<RefCell<StdRng>>,
    child_coro: Rc<RefCell<dyn ArbGen<ExprId> + Unpin>>,
    max_width: usize,
) -> impl ArbGen<ExprId> {
    #[coroutine]
    move || {
        let mut arb_vec_coro = arb_vec_of_rc_refcell_of(child_coro, Rc::clone(&rng), max_width);
        loop {
            let children_ids = match pin!(&mut arb_vec_coro).resume(()) {
                CoroutineState::Yielded(subexpr) => subexpr,
                CoroutineState::Complete(()) => return (),
            };

            let expr = Expr::Alt { children_ids };
            let expr_id = alloc(Rc::clone(&arena), expr);
            yield expr_id;
        }
    }
}

fn do_arb_expr_depth_1(
    arena: Rc<RefCell<Vec<Expr>>>,
    rng: Rc<RefCell<StdRng>>,
) -> Rc<RefCell<dyn ArbGen<ExprId> + Unpin>> {
    let coro = #[coroutine]
    move || {
        let mut term = arb_term(Rc::clone(&arena), Rc::clone(&rng));
        loop {
            let expr_id = match pin!(&mut term).resume(()) {
                CoroutineState::Yielded(child_id) => child_id,
                CoroutineState::Complete(()) => return (),
            };
            yield expr_id;
        }
    };
    Rc::new(RefCell::new(coro))
}

fn do_arb_expr_depth_n(
    arena: Rc<RefCell<Vec<Expr>>>,
    rng: Rc<RefCell<StdRng>>,
    max_width: usize,
    child_coro: Rc<RefCell<dyn ArbGen<ExprId> + Unpin>>,
) -> Rc<RefCell<dyn ArbGen<ExprId> + Unpin>> {
    let coro = #[coroutine]
    move || {
        let mut term = arb_term(Rc::clone(&arena), Rc::clone(&rng));
        let mut opt = arb_opt(Rc::clone(&arena), Rc::clone(&child_coro));
        let mut alt = arb_alt(
            Rc::clone(&arena),
            Rc::clone(&rng),
            Rc::clone(&child_coro),
            max_width,
        );

        let mut unexhausted_variants = vec![0, 1, 2];
        while unexhausted_variants.len() > 0 {
            let tree_node_variant = {
                let mut r = rng.borrow_mut();
                unexhausted_variants.choose(&mut r).unwrap()
            };
            let expr_id = match tree_node_variant {
                0 => match pin!(&mut term).resume(()) {
                    CoroutineState::Yielded(expr_id) => expr_id,
                    CoroutineState::Complete(()) => {
                        unexhausted_variants.remove(*tree_node_variant);
                        continue;
                    }
                },
                1 => match pin!(&mut opt).resume(()) {
                    CoroutineState::Yielded(expr_id) => expr_id,
                    CoroutineState::Complete(()) => {
                        unexhausted_variants.remove(*tree_node_variant);
                        continue;
                    }
                },
                2 => match pin!(&mut alt).resume(()) {
                    CoroutineState::Yielded(expr_id) => expr_id,
                    CoroutineState::Complete(()) => {
                        unexhausted_variants.remove(*tree_node_variant);
                        continue;
                    }
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
) -> impl ArbGen<ExprId> + Unpin {
    #[coroutine]
    move || {
        let mut coro = do_arb_expr_depth_1(Rc::clone(&arena), Rc::clone(&rng));
        for _ in 2..max_depth {
            coro = do_arb_expr_depth_n(Rc::clone(&arena), Rc::clone(&rng), max_width, coro);
        }
        loop {
            let expr_id = {
                let mut expr = coro.borrow_mut();
                let expr_id = match pin!(expr.deref_mut()).resume(()) {
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
    let rng = make_test_rng();
    const MAX_WIDTH: usize = 3;
    const MAX_DEPTH: usize = 3;
    let arena_rc = AssertUnwindSafe(Rc::clone(&arena));
    let arb = arb_expr(arena, rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify_with_reset(
        |t| {
            let arena_guard = arena_rc.borrow();
            get_max_tree_width(arena_guard.as_ref(), t) <= MAX_WIDTH
        },
        || {
            let mut arena_guard = arena_rc.borrow_mut();
            arena_guard.clear();
        },
        arb,
    ) {
        assert!(false, "{}", counterexample);
    }
}

#[test]
fn test_tree_depth_within_limits() {
    let arena = Rc::new(RefCell::new(Vec::new()));
    let rng = make_test_rng();
    const MAX_WIDTH: usize = 5;
    const MAX_DEPTH: usize = 10;
    let arena_rc = AssertUnwindSafe(Rc::clone(&arena));
    let arb = arb_expr(arena, rng, MAX_WIDTH, MAX_DEPTH);
    if let Some(counterexample) = falsify_with_reset(
        |t| {
            let arena_guard = arena_rc.borrow();
            get_max_tree_depth(arena_guard.as_ref(), t) <= MAX_DEPTH
        },
        || {
            let mut arena_guard = arena_rc.borrow_mut();
            arena_guard.clear();
        },
        arb,
    ) {
        assert!(false, "{}", counterexample);
    }
}
