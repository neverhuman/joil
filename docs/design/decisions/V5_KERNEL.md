# V5-08 kernel prototype decision

Status: admit only the demonstrated diagnostic CPU prototype; no global geometry/error or physical admission.

The plain f64 bounded DAG interpreter remains the reference. Fidget is pinned exactly to 0.5.1, with default features disabled and only its CPU JIT feature enabled. Its own VM is included by the core crate. GPU, GUI, mesher, solver, scripting and serialized bytecode features are disabled. Upstream describes the library as experimental; this status is retained in every KernelIdentity. Rust 1.95 is required by the pinned upstream manifest and matches the current project CI.

Pinned registry input SHA-256 values:

- fidget 0.5.1: `69298d7641ec13971a5359ab20961a92482eab6201c7a89a1372a4ceac1f6987`
- fidget-core 0.5.1: `b72374bb0a76f138aacd08b0611de2154cbb5eea0cf8251b11ed7bf3447e9a58`
- fidget-jit 0.5.1: `a26c5c6c91f9b913b265c4c9888ff73ee7dfa93e169b295799e63cbe128b1fed`

The public lockfile binds all transitive registry checksums. Existing locked package versions remain unchanged; duplicate-version dependency names become explicit. All new fetch/extraction happens in the worker's project-local Cargo home.

The prototype compares the original CT-35 fluid field, branch/rejoin subtraction, a five-member internal cylindrical lattice and a 0.1mm thin spherical shell. Each case has 90 declared finite points and a 2e-6 scalar comparison tolerance fixed in the test source. It records actual compilation time and maximum sampled disagreement. The first executed run observed maxima from 1.6e-9 through 8.2e-8 and compilation around 2..12ms. These are software diagnostic observations, not throughput/speedup or unsampled error bounds. Exact current-run values and source binding belong to the task receipt.

Mandatory controls refuse f64-to-f32 overflow/underflow, actual small-feature precision collapse, invalid source arithmetic even when dead-code elimination would remove it, operation/size exhaustion, and injected JIT/VM disagreement. The reference re-evaluates all source nodes before every candidate result; backend optimization cannot hide source failure.

Admission envelope: Linux x86_64, at most 1024 validated nodes, checked finite coordinates and constants, and only points that pass all three evaluations under an explicit tolerance. The candidate is not used for interval pruning, whole-domain distance/volume/topology certification, manufactured-solid evidence or physical qualification. Global error, CAD-loft equivalence, other architectures and production speed remain unresolved. Larger or unsupported inputs use an explicit Unsupported outcome; there is no silent fallback that relabels a failed candidate as accepted.
