# Design

 * No trait based dispatch (`Arbitrary`).  Generators and shrinkers are straightforward coroutines instead
 * If two tests need a different shape of the tested data type, just combine generators differently.  No newtype wrappers necessary.
 * No macros.  Test cases are composed of function calls, ifs and loops
 * Linear, top to bottom test code
 * Only implement shrinker when it actually provides value, i.e. a large enough falsifier is found and it needs to be shrunk
 * Panics are caught and treated as test failures subject to shrinking.  An `.unwrap()` in tested code is also a violation of tested properties
 * RNG seed value is printed on stderr.  Use `cargo test -- ... --nocapture` to grab it
 * RNG seed value is taken from `FALSIFY_SEED` environment variable if set.  It's used for test failure reproduction
 * Need a custom generator that produces only a subset of possible values of a given type?  Just copy paste
   the generic generator code and specialize for your requirements.  There's no "You wanted a banana but what
   you got was a gorilla holding the banana and the entire jungle."

# Guidelines for implementing generators/shrinkers

 * `arb_*()` coroutines should generate random values indefinitely and never return.  Even if some data type
   is small enough to generate all values exhaustively.  The coroutine may be used as a building block in
   testing a more complex data type.  Terminating, then, would result in terminating the "big" generator
   prematurely.
 * In contract to `arb_*()`, `shrink_*()` coroutines should finish once they can't shrink further.  It allows
   the caller to attempt a different shrinking strategy if the falsifier is still unmanageably large.
