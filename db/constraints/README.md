# Database constraints

Future schemas must enforce unique evidence identity, required provenance,
foreign-key integrity and explicit unit/basis metadata. Application checks do not
replace database constraints. Define CHECK constraints for valid state transitions
where possible, and transaction tests for cross-row invariants. Unknown or absent
physical observations never become qualified through a default. Every constraint
needs valid and invalid write tests, plus migration and rollback coverage.
