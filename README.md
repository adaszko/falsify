Under what assumptions the binary search shrinker finds the smallest falsifier?

Design:
 * No macros.  Explicity is better than implicit.
 * No `Arbitrary` trait.  It makes having many arbitrary value generators per type cumbersome as it requires introducing a newtype for each generator.
 * It's easy to vary inputs generation tactic between tests.  It's cumbersome with type-based dispatch
 * Test data generators are straightforward to implement.  No need to translate generation logic into a reified state machine in your head
 * You can postpone shrinker implementation until it's actually necessary (i.e. a large enough falsifier is found)
 * Go light on macros.  They obscure the testing logic
 * Treat panics as test failures by default (think `.unwrap()s`).
 * Diagnostic information (e.g. RNG seed value) is printed on stderr; use cargo test -- ... --nocapture to see
 * RNG seed value is configurable via `FALSIFY_SEED` environment variable for quick reproducibility

Arbitrary generator implementation guidelines:
 * It should produce values indefinitely

Design assumptions:
 * arb_*() coroutines should generate random values indefinitely and never return...
 * ...unless the data type is small enough to enumerate all values exhaustively, then the coroutine should return
 * shrink_*() coroutines should finish once they can't shrink further
