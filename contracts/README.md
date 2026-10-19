# Contract generation boundary

Existing authoritative schemas and Rust types remain in schemas/ and crates/.
This directory reserves generated interop output; it does not replace those sources.
Future generation must record source hashes, generator version and reproducible
command in a receipt. Review additive compatibility and negative input tests before
consumers update. A clean regeneration must have zero drift. No generator or new
client is claimed to exist by adding this boundary. The existing explorer types
remain subject to their current registry and manifest checks.
