# V5-12 expressiveness challenge registry

These 16 named cases are a frozen software challenge registry, not campaign seeds. The first 12 reproduce the CAD-10 concepts in the detailed S09 source; the final four are V5-12 additions. The current `crucible-genome` crate can validate and develop MPG records only. No geometry/backend compiler is registered here, so this registry does **not** claim physical realization or backend compilation. Cases requiring geometric witnesses remain blocked pending the V5-08/V5-09 compiler and independent observers.

`cases.json` binds each stable case ID to its independent witness and records the current executable boundary. `equal_shell_different_interior` is exercised in `crates/crucible-genome/tests/operators.rs`: distinct module topology produces distinct semantic subject identity even when the shell descriptor is the same.
