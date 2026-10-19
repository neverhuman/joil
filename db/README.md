# Database change discipline

This boundary reserves future persistence work; current scientific evidence stays
in its existing source-bound stores. No database service or migration is introduced.

A migration must name its source and destination schema, owner, compatibility
window and affected invariants. Test valid writes and rejection of invalid writes
against a disposable database. Review lock duration and backfill cost before use.
Take a backup and prove restore before a destructive change. Prefer an additive
expand/migrate/contract sequence. Rollback must preserve evidence identity; if an
inverse migration loses data, restore the verified backup instead. Stop on failed
constraints, partial backfills or mismatched receipt hashes. Record commands,
exit status and artifact hashes in a new receipt; never amend historical evidence.
