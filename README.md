Design:
 * Test data generators are straightforward to implement.  No need to translate generation logic into a reified state machine in your head
 * It's easy to vary inputs generation strategy between tests.  It's cumbersome with type-based dispatch
 * You can postpone shrinker implementation until it's actually necessary (i.e. a large enough falsifier is found)
 * Go light on macros.  They obscure the testing logic
 * The crate catches panics by default in order to treat .unwrap()s as test failures
 * Diagnostic information (e.g. RNG seed value) is printed on stderr; use cargo test -- ... --nocapture to see
 * RNG seed value is configurable via GENTEST_SEED environment variable for quick reproducibility

Design assumptions:
 * arb_*() coroutines should generate random values indefinitely and never return...
 * ...unless the data type is small enough to enumerate all values exhaustively, then the coroutine should return
 * shrink_*() coroutines should finish once they can't shrink further

Shrinker design:
 * Rust coroutines impose a restriction: `T` and `S` has to be the same type.

```Rust
let mut coroutine = #[coroutine] |t: T| {
    let s: S = yield 123;
};
```

Given that restriction, it's necessary for `S` to be `TestResult`.  This implies `T` also has to be
`TestResult`.  This results in a slightly more awkward API than necessary, namely the falsifier argument is
accepted by the custom-type shrinker, not the general `shrink()` function.
