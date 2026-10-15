# Draft Laboratory Rig Protocol v1 (`P06`): Single-Liner Paired A/B/A Testing of Removable CT-35 & Exotic Heads

**Protocol ID:** `EXP-RIG-JOIL-V1-DRAFT`  
**Status:** Draft experimental protocol (`P06`) pending formal preregistration and measured site/fluid inputs; no physical rig measurements have been executed yet.  
**Requirement Binding:** `requirements/RS-joil-research-v1.json`

---

## 1. Purpose and Scope

This draft protocol defines the physical bench verification required to advance a computationally screened removable hydrocyclone head (`ct35-chamber-wall-field-v1` or `ct35-exotic-head-v2`) from **Level 2 (Numerical Research Release)** to **Level 3 (Validated Design Engine)**.

All tests use a reusable downstream liner ($z \in [169.247, 1639.247]\text{ mm}$, comprising the $770\text{ mm}$ fine taper $D_2 = 35\text{ mm} \to D_3 = 17.5\text{ mm}$ and the $700\text{ mm}$ cylindrical tail at $D_3 = 17.5\text{ mm}$; full wetted length $1639.247\text{ mm}$) housed in an independent pressure-rated containment sleeve, swapping only the removable upper head ($z \in [0, 186.747]\text{ mm}$ with the frozen $0.5\,D_2 = 17.5\text{ mm}$ alignment collar).

---

## 2. Randomized Paired A/B/A Experimental Design & Independent Preparations

To eliminate drift in feed oil concentration $C_i$, temperature, dissolved air, or emulsification state across a test day, every candidate head $B_k$ is tested across **at least $n \ge 3$ independent fluid preparations and physical builds** (pilot minimum for variance/power estimation per `MASTER_PLAN.md` §7–8) in a **randomized blinded A/B/A sequence**:

1. **Block $A_1$ (Baseline CT-35 Reference Head):**
   - Sweep 5 feed flow rates $Q_f \in \{3.0, 3.5, 4.0, 4.5, 5.0\}\text{ m}^3/\text{h}$ at the frozen target reject split $r = Q_o / Q_f = 0.0200 \pm 0.0002$ (plus split sensitivity $r \in \{0.01, 0.02, 0.03, 0.05\}$ at $4.0\text{ m}^3/\text{h}$).
2. **Block $B_k$ (Candidate Exotic or Wall-Field Head):**
   - Swap the removable head inside the same pressure housing and downstream tail liner.
   - Repeat the exact 5-point flow sweep and 4-point reject-split sweep under identical feed preparation and backpressure control to $r = 0.0200 \pm 0.0002$.
3. **Block $A_2$ (Baseline CT-35 Repeat Verification):**
   - Re-install the CT-35 baseline head and repeat the nominal $Q_f = 4.0\text{ m}^3/\text{h}, r = 0.0200 \pm 0.0002$ operating point and bracketing flows ($3.0, 5.0\text{ m}^3/\text{h}$).
   - **Drift Gate:** If $|C_{u,A_2} - C_{u,A_1}| / C_{u,A_1} > 10\%$ or $|\Delta p_{u,A_2} - \Delta p_{u,A_1}| / \Delta p_{u,A_1} > 5\%$, the run is flagged as non-stationary and cannot certify a capacity gain.

---

## 3. Required Instrumentation and Metrology

1. **As-Built Geometry Metrology (Pre-Test Gate):**
   - Every printed removable head (`A` and `B_k`) is CT-scanned or CMM/bore-gauged before wet testing:
     - Overflow vortex-finder bore $D_o = 1.40\text{ mm}$ (pin gauge / optical comparator $\pm 0.02\text{ mm}$),
     - Collar inner diameter $D_2 = 35.00\text{ mm}$ at $z \in [169.25, 186.75]\text{ mm}$,
     - Dry mass $m_{\text{dry}}$ and surface roughness $R_a / k_s$ on wetted swirl-chamber walls.
2. **Hydraulic Instrumentation:**
   - Coriolis or calibrated electromagnetic flowmeters on feed ($Q_f$), underflow ($Q_u$), and overflow reject ($Q_o$); water closure gate $|Q_f - Q_u - Q_o| / Q_f \le 0.5\%$.
   - Differential pressure transducers ($\pm 0.25\%$ FS) measuring $\Delta p_u = p_{\text{inlet}} - p_{\text{underflow}}$ and $\Delta p_o = p_{\text{inlet}} - p_{\text{overflow}}$, allowing exact calculation of $\text{PDR} = \Delta p_o / \Delta p_u$.
3. **Dispersed Oil & Droplet Size Distribution (DSD) Metrology:**
   - **Inlet & Outlet DSD:** Inline laser diffraction / focused-beam reflectance or microscopic flow-cell imaging upstream of the head ($D_{50}, \sigma_g$ across $3\text{--}100\,\mu\text{m}$) and at the treated underflow sampling port.
   - **Concentration ($C_i, C_u, C_o$):** Triplicate isokinetic bottle samples per steady state (after $\ge 60\text{ s}$ hydraulic stabilization), analyzed via solvent extraction / UV-fluorescence or FTIR (reporting $\text{mg/L}$ dispersed oil, mean $\pm 95\%$ confidence interval).

---

## 4. Acceptance Criteria for Capacity Gain

A candidate head $B_k$ demonstrates a validated treated-water capacity improvement over CT-35 (`A`) only if:
1. Over the common or pressure-matched operating envelope, $C_u(B_k) \le 30\text{ mg/L}$ (upper 95% confidence bound) at a higher treated underflow flow rate $Q_u$ than CT-35 at matched $\Delta p_u$, **or** achieves a statistically significant reduction in $C_u$ ($p < 0.05$ across paired A/B/A blocks) at equal $Q_u$ and $\Delta p_u$.
2. Post-test endoscopic inspection shows zero collar leakage, zero structural cracking under differential pressure, and no unflushable oil-trapping dead zones.
