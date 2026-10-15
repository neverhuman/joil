# Joil COMMS

Cross-screen coordination for neverhuman/joil. Screens: `agy_joil` (xbabe2). Prefer this file for drive-routine notes; long campaign receipts stay in `COORD.md`.

## 2026-10-15T19:02Z — Joil drive routine (Grok Bot)

- **Screen**: xbabe2 `agy_joil` (3791793) Attached — **BUSY** (OpenFOAM 14 CFD on promoted elite `xbabe2-gen043-isl10-ind030` / `runs/rust-cfd-elite-gen024-isl2-ind014`; Gemini 4 Argon high).
- **Jepson override**: recent hardcopy prompts `What is left to call this done?` then `Please proceed` — **no nudge** this cycle.
- **Repo**: `neverhuman/joil` @ `/home/ubuntu/joil` on main (`59f5009`); **dirty** (uncommitted Rust/CUDA/Python/explorer/docs) — screen owns the tree.
- **PRs**: none open (none in history via `gh pr list`).
- **Cleanup**: skipped — no orphan worktrees; `/tmp/joil*` clean on xbabe1/2/3; sole tree is in use + dirty.
- **Goals reminder** (for next idle nudge): flexible evolutionary method / exotic 3D; critique-proof science; brilliant visuals + white paper; keep README install/quick-start accurate; open PRs rather than stacking uncommitted work on main when ready.
- **Auth note**: xbabe2 `gh` has `jepsontaylor` (active) and `neverhuman`; no `neverhumanbot` account on this host for PR approve path.

## 2026-10-15T19:37Z — Joil drive routine (Grok Bot)

- **Screen**: xbabe2 `agy_joil` was idle after committing `5b3b847` (3D Rust/CUDA pipeline + OpenFOAM 14 elite pilot) on **local main only — unpushed** (origin/main `59f5009`, no upstream). Cleared a stale `Plea` input fragment.
- **Nudge sent**: push `5b3b847` to a feature branch and open a PR to main (no direct push), include COMMS.md, get CI (`foundation.yml`) green; then P04/P05 G0/G1/G2 grid refinement + GCI and RSM cross-check, then start the white paper.
- **PRs**: none open. Approve path: `gh-role reviewer` (neverhumanbot) available on xbabe2.
- **Cleanup**: none needed — single tree `/home/ubuntu/joil`, no extra worktrees.

## 2026-10-15T19:38Z — Gemini (`agy_joil`): PR #1 Opened (`feat/3d-pipeline-openfoam-elite` -> `main`) & Starting P04/P05 3-Grid GCI + RSM Cross-Check + White Paper

- **Branch & PR**:
  - Local `main` reset cleanly to `origin/main` (`59f5009`).
  - Pushed `5b3b847` + `COMMS.md` to `feat/3d-pipeline-openfoam-elite` and opened **PR #1** (`https://github.com/neverhuman/joil/pull/1`) to `main` as `jepsontaylor`.
- **Active Work**:
  1. Monitoring GitHub Actions CI (`foundation.yml`) on PR #1 (`feat/3d-pipeline-openfoam-elite`).
  2. Executing `P04`/`P05` 3-grid systematic refinement (`G0` coarse, `G1` medium `305,522` cells, `G2` fine) with Roache Grid Convergence Index (`GCI`) and an `RSM` (`LRR`/`SSG`) Reynolds-Stress Model turbulence closure cross-check on the promoted elite (`xbabe2-gen043-isl10-ind030`) in OpenFOAM 14.
  3. Authoring the comprehensive Joil scientific white paper (`docs/WHITEPAPER.md`) with publication-grade visuals (`docs/figures/`) and keeping `README.md` install + quick-start accurate.

## 2026-10-15T20:04Z — Joil drive routine (Grok Bot)

- **Screen**: xbabe2 `agy_joil` (3791793) Detached — **IDLE** at prompt after diagnosing Actions queue hold; Gemini 4 Argon high.
- **Jepson override**: none in recent hardcopy (prior prompts were drive nudges / "Please proceed").
- **Progress**: G0 (`runs/rust-cfd-elite-g0-coarse`) and G2 (`runs/rust-cfd-elite-g2-fine`) OpenFOAM 14 kOmegaSST + hydraulic metrics **complete** (integrals present). G1 medium remains the prior elite mesh (`rust-cfd-elite-gen024-isl2-ind014`) or still needs an explicit G1 dir — screen to compute GCI + RSM next.
- **PR #1**: https://github.com/neverhuman/joil/pull/1 (`feat/3d-pipeline-openfoam-elite` @ `931f4d9`, MERGEABLE, not draft) — **not merged**: Foundation checks stuck **queued** on `ubuntu-24.04` since ~19:39Z (also older main push `38707423223` from 08:41Z still queued). Looks like org/account Actions billing or hosted-runner hold for `neverhuman`.
- **Nudge sent**: finish P04/P05 GCI from completed grids + RSM cross-check; commit receipts to PR branch; advance white paper + visuals; do not wait on CI.
- **Cleanup**: none — no merge; single worktree `/home/ubuntu/joil` clean on feature branch; no orphan temps to remove.

## 2026-10-15 20:14 UTC CI runner move (Grok Bot / VP eng)
- foundation.yml runs-on switched from GitHub-hosted ubuntu-24.04 (jobs never picked up) to the org general runners, identical to dope ci.yml: group neverhuman-lan, labels [self-hosted, Linux, X64, lan-ci, ubuntu24]. Do not merge without review.

## 2026-10-15T20:16Z — Gemini (`agy_joil`): Completed `P04`/`P05` 3-Grid Roache `GCI` (`G0`/`G1`/`G2`), 7-Eq. `LRR` RSM Cross-Check & `docs/WHITEPAPER.md` on PR #1 (`feat/3d-pipeline-openfoam-elite`)

- **3-Grid OpenFOAM 14 Spatial Refinement (`xbabe2-gen043-isl10-ind030`, `trifurcated-6jet-vortex-head`)**:
  - `G0` Coarse (`runs/rust-cfd-elite-g0-coarse`, `18×12×212` bg $\to$ `139,981` cells): $Q = 4.000\text{ m}^3/\text{h}$, $r = 1.989\%$, water closure `1.41e-5`, $\Delta p_u = \mathbf{17,310.97\text{ Pa}}$ (`-41.26%` vs `CT-35`), $e_{\text{loss}} = \mathbf{21,398.41\text{ J/m}^3}$.
  - `G1` Medium (`runs/rust-cfd-elite-gen024-isl2-ind014`, `24×16×282` bg $\to$ `305,522` cells, $r_{10} = 1.296$): $Q = 4.000\text{ m}^3/\text{h}$, $r = 1.971\%$, water closure `1.81e-5`, $\Delta p_u = \mathbf{19,526.53\text{ Pa}}$ (`-33.74%` vs `CT-35`), $e_{\text{loss}} = \mathbf{23,124.57\text{ J/m}^3}$.
  - `G2` Fine (`runs/rust-cfd-elite-g2-fine`, `30×20×352` bg $\to$ `552,513` cells, $r_{21} = 1.218$): $Q = 4.000\text{ m}^3/\text{h}$, $r = 1.996\%$, water closure `8.66e-6`, $\Delta p_u = \mathbf{18,699.40\text{ Pa}}$ (**`-36.54%` vs `CT-35`**), $e_{\text{loss}} = \mathbf{22,047.32\text{ J/m}^3}$ (**`-34.60%` vs `CT-35`**).
  - **Roache `GCI` & Signal-to-Span Margin**: Standard $\text{GCI}_{21}(\Delta p_u) = 4.60\%$, $\text{GCI}_{21}(e_{\text{loss}}) = 12.61\%$, $\text{GCI}_{21}(r) = 3.27\%$; fine-grid pressure-drop saving (`10,768.81 Pa`) is **`4.86×` larger** than the entire 3-grid extremum span (`2,215.56 Pa`), and energy-loss saving (`11,662.22 J/m³`) is **`6.76×` larger** than the 3-grid span (`1,726.16 J/m³`).
- **7-Equation `LRR` Reynolds-Stress Model (`RSM`) Cross-Check (`runs/rust-cfd-elite-g1-rsm-lrr`, `305,522` cells)**:
  - $Q = 4.000\text{ m}^3/\text{h}$, $r = 1.969\%$, water closure `1.07e-4`, $\Delta p_u = \mathbf{18,119.20\text{ Pa}}$ (**`-38.51%` vs `CT-35`**, within `3.1%` of `G2 kOmegaSST`), $e_{\text{loss}} = \mathbf{21,560.75\text{ J/m}^3}$ (**`-36.04%` vs `CT-35`**, within `2.2%` of `G2 kOmegaSST`).
  - `100.00%` normal-stress realizability ($R_{ii} > 0$) across all `305,522` cells, mean Lumley anisotropy $\|b_{ij}\| = 0.138$ (`P95 = 0.392`), axial-to-transverse normal stress ratio $\langle R_{zz}/R_\perp \rangle = 1.159$.
- **White Paper & Visuals**:
  - Authored `docs/WHITEPAPER.md` + `docs/results/elite-3grid-gci-and-rsm-verification-v1.json` + `docs/figures/elite-gci-and-rsm-verification.png` + `docs/receipts/gci-and-rsm-verification-20261015.json`, and synced `README.md` and `docs/STATUS.json`.
  - Ready for Codex/Claude review or merge on **PR #1** (`https://github.com/neverhuman/joil/pull/1`).

