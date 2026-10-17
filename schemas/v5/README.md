# Physical interchange

`physical-contracts.schema.json` defines strict structural shapes, including every nested object. `contracts.generated.ts` is generated from those definitions by `generate_contracts.py`. Report types are read-only outputs; neither JSON nor TypeScript can mint an integrator verdict. Rust private constructors and current source resolution enforce that boundary.

The seven valid examples are synthetic software fixtures, frozen by `cargo run --locked -p crucible-contract --example freeze_fixtures`. Regenerate schema/TypeScript/negative examples with `python3 schemas/v5/generate_contracts.py`; CI checks freshness and exercises all frozen fixtures through the production Rust decoder.

JSON Schema alone cannot detect duplicate keys lost during ordinary JSON parsing, signed zero, floating-point exactness, dimensional incompatibility, byte identity, revocation or physical validity. Always run the strict kernel decoder and semantic checks. The duplicate-key, negative-zero and wrong-dimension fixtures deliberately test these extra requirements. A structurally valid subject with unknown material/roughness is not a qualified subject. No physical measurement or performance evidence is supplied by these examples.
