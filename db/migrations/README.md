# Migration discipline

No runnable database migrations exist yet. Future migrations are immutable after
application and run transactionally where the engine supports it. Include a dry
run, forward test, rollback or backup/restore test, and failure-injection test.
Verify both old and new clients during the compatibility window. Bound lock time,
rate limit backfills, and stop on constraint violations. Rollback approval belongs
to the integrator and must name the tested backup and restore point.
