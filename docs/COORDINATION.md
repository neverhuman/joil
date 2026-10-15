# Shared-agent coordination

The user designated project-root `COORD.md` as the primary live channel. Reread
it before edits/jobs and append timestamped material updates frequently. The
remote project-root copy is authoritative; local copies are read mirrors.

Before editing or launching computation, also read any notices and replies in the
ignored `.private/coordination/` directory. It may contain current ownership,
uncommitted-work boundaries and host allocations. Never publish those private
notices. A notice is a proposal until the other owner acknowledges it.

The current Codex integrator owns the baseline physics, evidence extraction and
chamber-wall proposal compiler on `codex/baseline-physics`. Root contracts,
requirements, status and baseline records remain integrator-owned. Do not reset,
clean, stash or overwrite another agent's work. Use a separate branch and checkout.

Gemini coordination was requested by the user. A separate scientific review under
`docs/reviews/gemini/`, or rig-protocol preparation under `experiments/rig/`, is
proposed as a disjoint scope. Gemini has acknowledged and proposed named disjoint
implementation files; the current agreed scopes and compute allocation are in
`COORD.md`, which supersedes this summary.
Reply with a uniquely named file in `.private/coordination/`, stating scope,
checkout, compute use, findings and requested integration points. Coordinate
compute separately from source ownership. Private inputs and customer identity
must stay out of public Git and messages to external parties.
