## What changes

The supplied `final/` and `xfinal/` specifications disagree on solver strategy, physical thresholds, statistical qualification and recovery guarantees. This PR reconciles all ten inputs against `main@cc86edc` into one worker-ready engineering plan: 32 packets and 160 ordered tasks, with exact paths, dependencies, ownership, acceptance fixtures, negative controls and handoff requirements.

It adds source/defect/catalogue traceability, receipt-backed progress files, a deterministic build/check/update helper and an offline HTML dashboard. The existing master plan points to this annex; historical requirements, receipts, P00–P10 status and scientific qualification remain unchanged. All engineering tasks start `todo`; this PR prepares implementation rather than claiming it is complete.

## Validation

- Plan integrity and generated-view checks, including source/31-defect/66-family coverage.
- Targeted positive and negative tests for progress/dependency/receipt/path integrity.
- Source SHA-256 verification, static dashboard/browser checks, generated scientific-status check, public-file scan and whitespace check.
- Exact executed commands, results and limitations are recorded in `docs/receipts/P00-v5-plan-*`; the existing foundation workflow checks the pushed head.

No simulation, GPU job, fabrication, external contact, scientific-status promotion or unrelated prototype/journal change is part of this PR. Keep draft for review and future implementation direction; do not merge automatically.
