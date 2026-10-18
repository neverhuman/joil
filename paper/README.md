# Query-built paper fixtures

`cargo run --offline --locked -p crucible-paper --bin paper-build` builds the complete negative-result fixture from exact typed observation queries. `PaperBundle::validate` rejects any mismatch in observation identity, subject, scenario, estimand, unit, denominator, current tier, or allowed interpretation. Figures must name observations and a generator/source digest.

`dependency-license-inventory.json` records all resolved Cargo packages and declared licenses; thirteen workspace packages currently have no declared license and need owner review before redistribution. `dependency-prior-design-inventory.json` records the CT-35 comparator and the ten source-review inputs. These are engineering review inventories, not legal opinions or novelty searches.

The fixture is synthetic software evidence. It intentionally contains no performance claim. Historical `docs/WHITEPAPER.md` and existing correction records remain immutable inputs; this new build does not rewrite them.
