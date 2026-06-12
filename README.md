Design:
 * Test data generators are straightforward to implement.  No need to translate generation logic into a reified state machine in your head
 * It's easy to vary inputs generation strategy between tests.  It's cumbersome with type-based dispatch
 * You can postpone shrinker implementation until it's actually necessary (i.e. a large enough falsifier is found)
 * Go light on macros.  They obscure the testing logic

Design assumptions:
 * arb_*() coroutines should generate random values indefinitely and never return...
 * ...unless the data type is small enough to enumerate all values exhaustively, then the coroutine should return
 * shrink_*() coroutines should finish once they can't shrink further
