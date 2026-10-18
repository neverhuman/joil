# G008 recovery timing fixture follow-up

Foundation run 39104878394 on the pushed 05d1855 failed the recovery timing
positive; the next mutex-using test failed from poison. Seven other suites
passed. Retain that actual CI failure separately from the successful local gate.

The timing fixture put a second whole-binary hash read inside a Python child,
alongside its 80 ms deliberate delay. That work was subject to the unchanged
300 ms native deadline. Its assumed latency is not portable; moving redundant
producer verification into charged coordinator preparation is a test-fixture
repair, not proof that CI's elapsed time had one particular cause.

The integrator may edit only `crates/crucible-runtime/tests/recovery.rs` for this
repair and add its evidence receipt. Preserve the 300 ms execution timeout,
700 ms preparation delay, 80 ms positive delay and 600 ms negative delay. Verify
the actual Rust producer bytes during the preparation callback, and keep the
wrapper itself content-bound. Keep real child execution, cancellation, charging
and observation checks. Do not move the production deadline or relax an assertion.

An explicit temporary early-clock mutation in `src/task.rs` may be used only as
a recorded negative control, restored byte-for-byte before committing. The
corrected fixture must detect that mutation and pass on unchanged production
timing code. Source/test hashes and command outcomes must identify both runs.
Full integration gates and exact-head CI remain required; V5-07-05 stays in review.
