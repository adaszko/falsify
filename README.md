Under what assumptions the binary search shrinker finds the smallest falsifier?

Design:
 * Tested data types don't need to implement any special traits like `Arbitrary`
 * No macros.  The crate's logic is simple.  Macros obscure it.
 * Test data generators are straightforward to implement.  The yak to shave is substantially smaller than with reified state machines.
 * Only implement shrinker when it actually provides value (i.e. a large enough falsifier is found)
 * Panics are considered test failures
 * Diagnostic information (e.g. RNG seed value) is printed on stderr; use `cargo test -- ... --nocapture` to see
 * RNG seed value is configurable via `FALSIFY_SEED` environment variable for quick reproducibility

Arbitrary generator implementation guidelines:
 * It should produce values indefinitely

Design assumptions:
 * arb_*() coroutines should generate random values indefinitely and never return...
 * ...unless the data type is small enough to enumerate all values exhaustively, then the coroutine should return
 * shrink_*() coroutines should finish once they can't shrink further
