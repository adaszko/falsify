# Design

 * No trait based dispatch (`Arbitrary`).  Generators and shrinkers are straightforward coroutines instead
 * If two tests need a different shape of the tested data type, just combine generators differently.  No awkward newtype wrappers necessary.
 * No macros.  Test cases are composed of function calls, ifs and loops
 * Only implement shrinker when it actually provides value, i.e. a large enough falsifier is found and it needs to be shrunk
 * Panics are caught and treated as test failures subject to shrinking.  An `.unwrap()` in tested code is also a violation of tested properties
 * RNG seed value is printed on stderr.  Use `cargo test -- ... --nocapture` to grab it
 * RNG seed value is taken from `FALSIFY_SEED` environment variable if set.  It's used for test failure reproduction

# Guidelines for implementing generators/shrinkers

 * `arb_*()` coroutines should generate random values indefinitely and never return.  Even if some data type
   is small enough to generate all values exhaustively, the coroutine may be used as a building block in
   testing a more complex data type.  Terminating, then, would result in terminating the "big" generator
   prematurely.
 * `shrink_*()` coroutines should finish once they can't shrink further.  It allows the caller to attempt a
   different shrinking strategy if the falsifier is still unmanageably large.
