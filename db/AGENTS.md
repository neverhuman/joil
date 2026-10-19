# Database boundary

The integrator owns this boundary. No database is deployed by these declarations.
Keep database access behind the service layer; the explorer must never issue SQL.
Use migrations for every schema change. Declare uniqueness, foreign keys, check
constraints, nullability and transaction invariants in db/constraints before use.
Never weaken a constraint to admit invalid evidence. Read db/README.md first.
