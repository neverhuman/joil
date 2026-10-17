# Task completion receipts

Add immutable uniquely named JSON receipts here when implementation actually completes. No engineering task is complete in the initial planning package. Raw logs, solver fields and protected geometry remain in private storage; publish only deliberately reviewed references/digests.

Required fields: `task_ids`, full `tested_commit`, `changed_paths`, `commands` with `argv`, zero `exit_code` and `log_sha256`, nonempty `positive_checks` and `negative_checks`, nonempty `artifacts` with SHA-256, `resource_allocation`, `limitations`, `next_dependency`. The progress file stores the receipt’s relative path and exact digest. Negative-control commands should exit zero after asserting that the planted bad input was rejected.

The planning helper checks structure, referenced-file integrity, ordering and dependencies. It cannot independently prove the receipt’s scientific assertions. Reviewer/evidence-registry admission is still required. Never edit an already referenced receipt to change a result; publish a correction and update dependencies explicitly.
