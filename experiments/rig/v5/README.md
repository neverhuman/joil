# V5 physical confirmation protocol

`protocol.json` is an unsigned planning draft. It is not preregistered or approved and does not authorize fabrication, purchases, lab scheduling or external contact. No physical rig data are included. Independent pressure containment and named responsible-engineer review are prerequisites.

The comparator, common-duty policy and decision profile must be frozen before any confirmation data collection. Power derives from strictly identified independent pilot pairs; repeated measurements, nominal repeats from one build, and repeated subsamples do not increase independent `n`. Independent-unit and coded-specimen order is randomized and blinded; allocation stays concealed. Calibration controls live in a separate manifest from the sealed holdout manifest.

`analysis/validation/v5/protocol.py` is a deterministic software implementation for synthetic controls. It requires clock-sync, recovery-control, per-sensor calibration and per-sensor/sample-line lag evidence hashes. It corrects instrument and sample-line delay, aligns every channel to common windows, separates independent sample uncertainty from common calibration/systematic uncertainty, propagates both through synchronized concentration-flow mass integrals, and refuses extrapolation or censored windows. It is not the canonical DecisionProfile evaluator or evidence registry.
