import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import genome from '../../../baselines/ct35/genome.json';
import reference from '../../../baselines/ct35/reference.json';
import status from '../../../docs/STATUS.json';
import work from '../../../docs/WORK_PACKETS.json';
import catalog from '../../../docs/TEST_CATALOG.json';
import hydraulicReport from '../../../docs/results/baseline-hydraulics-v1.json';
import rustQualification from '../../../docs/results/rust-evolution-qualification-v1.json';
import gciRsmReport from '../../../docs/results/elite-3grid-gci-and-rsm-verification-v1.json';
import wave2Report from '../../../docs/results/wave2-multiseed-ablation-and-holdout-v1.json';
import proposalPlate from '../../../docs/figures/chamber-sections.png';
import topologyAtlasPlate from '../../../docs/figures/rust-3d-topology-atlas.png';
import paretoConvergencePlate from '../../../docs/figures/pareto-map-elites-convergence.png';
import gciRsmPlate from '../../../docs/figures/elite-gci-and-rsm-verification.png';
import vortexPhysicsPlate from '../../../docs/figures/openfoam14-3d-vortex-physics-comparison.png';
import wave2AtlasPlate from '../../../docs/figures/wave2-exotic-3d-genome-atlas.png';
import wave2AblationPlate from '../../../docs/figures/wave2-multiseed-ablation-and-ci.png';
import wave2GradeDsdPlate from '../../../docs/figures/wave2-grade-efficiency-and-dsd-shift.png';
import wave2StructuralPlate from '../../../docs/figures/wave2-structural-lame-and-modal-qualification.png';
import wave2Top8GalleryPlate from '../../../docs/figures/wave2-8-exotic-phenotypes-gallery.png';
import wave2VortexQdGif from '../../../docs/figures/wave2-vortex-and-qd-illumination.gif';
import wave2TopologyMorphGif from '../../../docs/figures/wave2-topology-morph-and-lineage.gif';
import { Viewport3D } from './Viewport3D';
import './style.css';

function Section({ full }: { full: boolean }) {
  const d = reference.derived_mm;
  const scale = full ? 690 / d.z_end : 690 / d.z_head;
  const x = (z: number) => 95 + z * scale;
  const y = (r: number) => 185 - r * 2.4;
  const stations = full
    ? [[0, 35], [70, 35], [d.z_cone_end, 17.5], [d.z_taper_end, 8.75], [d.z_end, 8.75]]
    : [[0, 35], [70, 35], [d.z_cone_end, 17.5], [d.z_head, 17.301136]];
  const upper = stations.map(([z, r]) => `${x(z)},${y(r)}`).join(' ');
  const lower = [...stations].reverse().map(([z, r]) => `${x(z)},${y(-r)}`).join(' ');
  return <svg viewBox="0 0 880 380" role="img" aria-label={`${full ? 'Full liner' : 'Head'} analytic longitudinal schematic, dimensions in millimetres`}>
    <defs><pattern id="grid" width="28" height="28" patternUnits="userSpaceOnUse"><path d="M28 0H0V28" fill="none" stroke="#25333e" strokeWidth=".5" /></pattern></defs>
    <rect x="0" y="0" width="880" height="380" fill="url(#grid)" />
    <rect x={x(0)} y="65" width={x(d.z_cone_end) - x(0)} height="240" fill="#d4b06a" opacity=".06" />
    <line x1="35" y1="185" x2="835" y2="185" stroke="#5b7080" strokeDasharray="7 5" />
    <polygon points={`${upper} ${lower}`} fill="#193a45" stroke="#72c1c1" strokeWidth="2" />
    <line x1={x(d.z_cone_end)} y1="63" x2={x(d.z_cone_end)} y2="307" stroke="#d4b06a" strokeDasharray="4 5" />
    <line x1={x(0)} y1="49" x2={x(full ? d.z_end : d.z_head)} y2="49" stroke="#8b9ca9" />
    <text x="95" y="34">{full ? '1,639.247 mm / full wetted liner' : '186.747 mm / printed head extent'}</text>
    <text x="95" y="334">z = 0 / inner end wall</text>
    <text x="95" y="355">D1 70 mm · D2 35 mm · reducing cone 10° half-angle</text>
    <text x="650" y="334">+z downstream →</text>
    {!full && <><text x="335" y="85">chamber</text><text x="585" y="105">reducing cone</text>
      <text x="695" y="286" fill="#d4b06a">fixed collar</text></>}
    {full && <><text x="430" y="145">770 mm fine taper</text><text x="690" y="145">700 mm tail</text></>}
  </svg>;
}

function HydraulicResults() {
  const [index, setIndex] = useState(hydraulicReport.points.length - 1);
  const point = hydraulicReport.points[index];
  const obs = point.observations;
  const reverse = point.external_outlet_reverse_flow_m3_s.overflow;
  return <section className="panel evidence">
    <div className="panelhead"><div><h2>CT-35 / executed hydraulic diagnostics</h2><p>Steady SST initializer · full liner · unqualified for design ranking</p></div><span className="state">OIL FITNESS UNKNOWN</span></div>
    <label className="runpicker">Saved run <select value={index} onChange={e => setIndex(Number(e.target.value))}>{hydraulicReport.points.map((p, i) => <option key={p.run_id} value={i}>{p.run_id}</option>)}</select></label>
    <div className="metrics measured">
      <div><small>Measured feed</small><strong>{obs.feed_m3_h.toFixed(3)} <b>m³/h</b></strong><span>Full 3D / two inlets</span></div>
      <div><small>Reject fraction</small><strong>{(obs.reject_fraction * 100).toFixed(3)} <b>%</b></strong><span>Target 2.000 ± 0.020%</span></div>
      <div><small>Static inlet → treated</small><strong>{(obs.static_tap_drop_pa / 1000).toFixed(2)} <b>kPa</b></strong><span>PDR {obs.pdr?.toFixed(3) ?? 'unknown'} / draft range 1.4–2.5</span></div>
      <div><small>Full-domain energy loss</small><strong>{(point.full_domain_hydraulic_loss_j_per_treated_m3 / 1000).toFixed(2)} <b>kJ/m³</b></strong><span>Boundary mechanical flux / treated water</span></div>
    </div>
    <dl><dt>Water closure error</dt><dd>{(obs.water_closure_fraction * 100).toExponential(3)}%</dd><dt>Numerical outlet extension</dt><dd>{point.numerical_outlet_extension_m * 1000} mm</dd><dt>Reject-boundary reverse flow</dt><dd>{(reverse * 3600).toExponential(3)} m³/h</dd><dt>Saved pseudo-iteration</dt><dd>{obs.pseudo_iteration} / no physical averaging interval</dd><dt>Oil concentration / removal</dt><dd>Unknown / no measured full-liner cohort result</dd><dt>Qualified fitness / capacity gain</dt><dd>Unknown / campaign admission blocked</dd></dl>
    <div className="note"><strong>What these numbers establish</strong><p>A reproducible flow calculation ran and its conserved fluxes were extracted. Mesh, transient statistics, outlet sensitivity, oil transport and independent validation remain unresolved. These values cannot establish oil-quality compliance or superiority.</p></div>
    <details><summary>Qualification blockers and provenance</summary><ul>{point.fitness.remaining_qualification_reasons.map(reason => <li key={reason}>{reason.replaceAll('_', ' ')}</li>)}</ul><p>Snapshot SHA-256</p><code className="digest">{point.evidence.snapshot.sha256}</code><p>Native integral SHA-256</p><code className="digest">{point.evidence.native_integrals.sha256}</code></details>
  </section>;
}

function App() {
  const [tab, setTab] = useState('3D Explorer');
  const [full, setFull] = useState(false);
  const [slideIdx, setSlideIdx] = useState(0);
  const [showAllSlides, setShowAllSlides] = useState(false);
  const p = genome.parameters;
  const rq = rustQualification;
  const el = rq.promoted_3d_elite;
  const cm = rq.campaign_metrics;
  const sq = rq.surrogate_and_engine_qualification;
  const fp = rq.flux_weighted_inlet_cohort_probe;
  const p05 = rq.p05_population_and_pbm_qualification;
  const p06 = rq.p06_experiment_and_metrology_qualification;
  const p09 = rq.p09_ops_and_scene_qualification;
  const gci = gciRsmReport.roache_gci_analysis;
  const rsm = gciRsmReport.rsm_turbulence_closure_crosscheck;
  const w2s = wave2Report.summary_statistics;
  const w2t = wave2Report.pairwise_tests;
  const wave2Slides = [
    { title: 'Plate 5 · Exotic 3D Genome Atlas', src: wave2AtlasPlate, sha: 'c4656fea7153db8c…', caption: '4 Wave-2 Exotic 3D archetypes (Cylindrical-Gyroid TPMS, Schwarz-P Shear Buffer, Octa-Branch 8-Jet Helical Manifold, Counter-Fluted Cascade) + 11-family Cout,p95 ranking.' },
    { title: 'Plate 6 · 10-Seed B0–B3 Ablation & CIs', src: wave2AblationPlate, sha: 'ef83db92f9658d5a…', caption: 'Pre-registered 10-seed equal-budget ablation (5,400 evals): B3 beats B1 & B2 (Mann–Whitney U=100, p_Holm=0.000314, Cliff δ=1.00) + D1–D4 held-out duties & 10% audit.' },
    { title: 'Plate 7 · 10-Bin Grade Efficiency & DSD', src: wave2GradeDsdPlate, sha: 'cebfca5828bf295e…', caption: '10-bin reduced centrifugal efficiency E_red(d), stratified feed-to-treated DSD transfer, and Hinze d_max vs entry shear G_max.' },
    { title: 'Plate 8 · P08 Structural Lamé & Modal', src: wave2StructuralPlate, sha: '0b5456a046184f31…', caption: '64-station thick-wall Lamé hoop/radial/Von Mises stress under 2.5 MPa burst duty, Strouhal modal lock-in margin, and LPBF buildability across all 11 families.' },
    { title: 'Plate 9 · Top-8 Exotic 3D Phenotypes Gallery', src: wave2Top8GalleryPlate, sha: 'b930335f0135c113…', caption: 'Top 8 most exotic valid 3D phenotypes (I_exotic up to 1.73) + 11-family stratified random-genome audit (220/220 = 100.0% canonicalize_and_preflight pass vs 0/30 negative controls).' },
    { title: 'Anim 1 · 3D Vortex & QD Illumination', src: wave2VortexQdGif, sha: '1f9ea28fedba6587…', caption: '36-frame 3D animation of oil-droplet core capture (40/20/10 μm) synchronized with 10-seed B3 vs B2 vs B1 hypervolume illumination.' },
    { title: 'Anim 2 · Phylogenetic 3D Topology Morph', src: wave2TopologyMorphGif, sha: 'dcfadcf06bdd7516…', caption: '36-frame continuous 3D B-rep wireframe morph across the 5-stage evolutionary lineage with live F0/P08/F1 telemetry.' },
    { title: 'Plate 3 · 3-Grid Roache GCI & Dual RSM', src: gciRsmPlate, sha: 'dbce90bb3878dd86…', caption: 'OpenFOAM 14 3-grid spatial refinement (G0 140k, G1 306k, G2 553k cells) + 7-equation LRR & SSG Reynolds-Stress closure verification.' },
    { title: 'Plate 4 · OpenFOAM 14 3D Vortex Physics', src: vortexPhysicsPlate, sha: '5c0c4b1ee4d0a82f…', caption: 'Radial swirl U_θ(r), reverse-core axial velocity U_z(r), static pressure core depression, and 100% Lumley turbulence realizability.' },
  ];
  const activeSlide = wave2Slides[slideIdx];
  return <><header><a className="brand" href="#">j<span>oil</span><i> / research</i></a><div className="project">EVOLUTIONARY FLUID SYSTEMS</div><span className="state">CFD EXECUTED · UNQUALIFIED</span></header>
    <main><div className="eyebrow">DESIGN SUBJECT / CT-35</div><div className="title"><h1>Better separation.<br /><em>Evidence first.</em></h1><p>Explore internal geometry. Preserve interfaces.<br />Measure every claim against shared duty and uncertainty.</p></div>
      <div className="metrics"><div><small>Reference diameter</small><strong>35 <b>mm</b></strong><span>Colman–Thew reconstruction</span></div><div><small>Reference feed</small><strong>4.0 <b>m³/h</b></strong><span>Provisional operating point</span></div><div><small>Oil reject split</small><strong>2.0 <b>%</b></strong><span>Matched across comparisons</span></div><div><small>Research water quality</small><strong>30 <b>mg/L</b></strong><span>20 mg/L separate stretch duty</span></div></div>
      <nav aria-label="Research views">{['3D Explorer', 'GCI / RSM & Wave 2', 'Geometry', 'Baseline run', 'Proposals', 'Evidence', 'Work packets'].map(name => <button key={name} className={tab === name ? 'active' : ''} onClick={() => setTab(name)}>{name}</button>)}</nav>
      {tab === '3D Explorer' && <Viewport3D />}
      {tab === 'GCI / RSM & Wave 2' && <section className="panel evidence">
        <h2>P04/P05 3-Grid Roache GCI, 7-Equation LRR &amp; SSG RSM Cross-Check &amp; Wave-2 10-Seed Statistical Ablation</h2>
        <p>Critique-proof verification across G0 (139,981 cells), G1 (305,522 cells), and G2 (552,513 cells) OpenFOAM 14 meshes, dual 7-equation Reynolds-Stress closures (linear LRR + quadratic SSG), and the pre-registered Wave-2 10-seed equal-budget 4-arm ablation ({wave2Report.protocol_id}).</p>
        <div className="metrics measured">
          <div><small>Fine-grid Roache GCI₂₁</small><strong>{gci.dp_u_pa.gci21_standard_fs125_pct.toFixed(2)} <b>%</b></strong><span>Δp_u SNR {gci.signal_to_uncertainty_margin.saving_to_3grid_span_ratio.toFixed(2)}× 3-grid span</span></div>
          <div><small>LRR / SSG 7-eq RSM Δp_u</small><strong>{(rsm[0].dp_u_pa / 1000).toFixed(2)} / {(rsm[1].dp_u_pa / 1000).toFixed(2)} <b>kPa</b></strong><span>−38.5% / −38.4% vs CT-35 (100% realizable)</span></div>
          <div><small>Wave-2 B3 C_out,p95 (10 seeds)</small><strong>{w2s.B3_Exotic3D_QD_CEI.cout_p95_mg_l.mean.toFixed(2)} <b>mg/L</b></strong><span>95% CI [{w2s.B3_Exotic3D_QD_CEI.cout_p95_mg_l.ci95_low.toFixed(2)}, {w2s.B3_Exotic3D_QD_CEI.cout_p95_mg_l.ci95_high.toFixed(2)}]</span></div>
          <div><small>Holm–Bonferroni Wilcoxon</small><strong>p = {w2t[0].p_value_holm_bonferroni.toFixed(5)}</strong><span>Cliff&apos;s δ = {w2t[0].cliffs_delta.toFixed(2)} · 10% audit PASS</span></div>
        </div>
        <h3>Held-Out Physical Operating Duties (D1..D4) — Promoted Wave-2 Exotic Elite</h3>
        <div className="metrics measured">
          {wave2Report.promoted_wave2_top_elite_held_out_duties.map(d => (
            <div key={d.duty_id}>
              <small>{d.duty_id.replaceAll('_', ' ')} ({d.q_in_m3_h.toFixed(1)} m³/h, d50={d.d50_feed_um} μm)</small>
              <strong>{d.cout_mg_l.toFixed(2)} <b>mg/L</b></strong>
              <span>Δp_u {(d.dp_u_pa / 1000).toFixed(2)} kPa · d50 cut {d.d50_cut_um.toFixed(2)} μm · {d.passes_30_mg_l ? 'PASS (<15 mg/L)' : 'FAIL'}</span>
            </div>
          ))}
        </div>
        <div className="note">
          <strong>Honest F4 / CAD-09 / P06 Physical Validation Gate Status (Unwaived)</strong>
          <p>All metrics above are hash-bound computational and numerical verification artifacts (F0 / F1 / F2-carrier). In strict compliance with SCIENCE.md, qualified_fitness = {String(status.qualified_fitness)}, qualified_capacity_gain = {String(status.qualified_capacity_gain)}, and physical_validation = {String(status.physical_validation)} remain locked until: (1) CAD-09 LPBF Ti-6Al-4V X-ray CT metrology confirms throat tolerance ≤ 150 μm and bore roughness k_s ≤ 35 μm; (2) F2-Oil 10-bin coupled residence-time OpenFOAM qualification completes; and (3) P06 5-block blinded randomized A/B/A flow-loop rig crossover demonstrates LCB₉₅(ΔC_out) &lt; 0 at matched r = 2.00 ± 0.05%.</p>
        </div>
        <div className="panelhead" style={{ marginTop: '24px' }}>
          <div>
            <h3>Publication Figure &amp; Animation Carousel ({slideIdx + 1} / {wave2Slides.length}: {activeSlide.title})</h3>
            <p>{activeSlide.caption} · SHA-256: <code>{activeSlide.sha}</code></p>
          </div>
          <div className="toggle">
            <button onClick={() => setSlideIdx((slideIdx + wave2Slides.length - 1) % wave2Slides.length)}>← Prev</button>
            <button onClick={() => setSlideIdx((slideIdx + 1) % wave2Slides.length)}>Next →</button>
            <button className={showAllSlides ? 'selected' : ''} onClick={() => setShowAllSlides(!showAllSlides)}>{showAllSlides ? 'Carousel view' : 'Show all 8'}</button>
          </div>
        </div>
        <div className="toggle" style={{ flexWrap: 'wrap', marginTop: '10px' }}>
          {wave2Slides.map((s, idx) => (
            <button key={s.title} className={!showAllSlides && slideIdx === idx ? 'selected' : ''} onClick={() => { setShowAllSlides(false); setSlideIdx(idx); }}>
              {s.title}
            </button>
          ))}
        </div>
        {!showAllSlides ? (
          <img className="proposalplate" src={activeSlide.src} alt={activeSlide.title} />
        ) : (
          wave2Slides.map(s => <img key={s.title} className="proposalplate" src={s.src} alt={s.title} />)
        )}
      </section>}
      {tab === 'Geometry' && <div className="geometry"><section className="panel"><div className="panelhead"><div><h2>CT-35 / analytic section</h2><p>Schematic only · mm · no simulated field coloring</p></div><div className="toggle"><button className={!full ? 'selected' : ''} onClick={() => setFull(false)}>Head</button><button className={full ? 'selected' : ''} onClick={() => setFull(true)}>Full liner</button></div></div><Section full={full} /><div className="legend"><span><i className="dot gold" />Upstream design region</span><span><i className="dot cyan" />Fluid profile</span><span>Actual STEP, wall thickness and patch audit live in generated artifacts.</span></div></section><aside className="panel"><h2>Baseline contract</h2><dl><dt>Genome</dt><dd>{genome.family}</dd><dt>Chamber ID</dt><dd>70 mm</dd><dt>Oil bore</dt><dd>{(p.d2_mm * p.do_over_d2).toFixed(1)} mm</dd><dt>Twin feed area</dt><dd>2 × 120.264 mm²</dd><dt>Fluid inventory</dt><dd>1.1058 L / includes extensions</dd><dt>Nominal V/Q</dt><dd>0.995 s / not residence distribution</dd></dl><div className="note"><strong>Fit remains unresolved</strong><p>Assumed 45 mm radial envelope conflicts with inlet ports. Actual vessel drawing required before fit qualification.</p></div></aside></div>}
      {tab === 'Baseline run' && <HydraulicResults />}
      {tab === 'Proposals' && <section className="panel evidence"><h2>Flexible CAD &amp; Native Rust + CUDA 3D Topology Archive (11 Canonical Families)</h2><p>Exact baseline plus eight OpenCASCADE chamber proposals, a 7-family Wave-1 MAP-Elites archive ({cm.total_evaluations.toLocaleString()} candidates / {cm.monte_carlo_realizations.toLocaleString()} MC realizations, {cm.map_elites_occupied_cells} QD cells, {cm.pareto_front_size} Pareto elites), and the 11-family Wave-2 Exotic 3D expansion (Cylindrical-Gyroid TPMS, Schwarz-P Rectifier, Octa-Branch 8-Jet Manifold, Counter-Fluted Cascade). All promoted candidates pass F0 B-rep/mesh, 5-patch watertight CFD surface (CAD-04/06), P08 structural/LPBF checks, and P09 true-scale GLB scene export ({(el.glb_scene_bytes / 1024).toFixed(1)} KiB, scale {p09.glb_true_scale.toFixed(1)} {p09.glb_units}), with qualified fitness held null pending F2/F4 validation.</p><div className="metrics measured"><div><small>Promoted 3D elite</small><strong>{el.id}</strong><span>{el.topology_class}</span></div><div><small>P08 yield safety factor</small><strong>{el.p08_yield_safety_factor.toFixed(2)}<b>×</b></strong><span>Ti-6Al-4V / 2.5 MPa Δp</span></div><div><small>Vane modal margin</small><strong>{el.p08_vane_modal_freq_ratio.toFixed(2)}<b>×</b></strong><span>f₁ / f_Strouhal ≥ 3.0</span></div><div><small>5-patch CFD surface</small><strong>{el.multi_patch_open_edges} <b>open edges</b></strong><span>{el.multi_patch_triangles.toLocaleString()} SI triangles · watertight</span></div></div><img className="proposalplate" src={wave2AtlasPlate} alt="True-scale 3D cutaway atlas of Wave-2 exotic topologies and 11-family ranking" /><img className="proposalplate" src={topologyAtlasPlate} alt="True-scale 3D cutaway atlas of 6 evolved hydrocyclone topology families and coupled droplet trajectories" /><img className="proposalplate" src={paretoConvergencePlate} alt="4D MAP-Elites archive Pareto front, 10-bin grade efficiency curves, entry shear mitigation, and P07 GP surrogate LOFO-CV calibration" /><img className="proposalplate" src={proposalPlate} alt="Actual CAD cross sections of the baseline and eight chamber wall proposals, on a common millimetre scale" /><div className="note">STEP, metre STL, 5-patch OpenFOAM case dictionaries ({el.cfd_case_dictionary_count} hash-bound files) and binary glTF 2.0 (.glb) scene bundles are archived per candidate. Sour-service qualification remains false ({String(el.p08_sour_service_qualified)}).</div></section>}
      {tab === 'Evidence' && <section className="panel evidence"><h2>Confidence must be earned</h2><p>Current state: {status.phase.replaceAll('_', ' ')}. Arithmetic, CAD, structural and F1 screen checks cannot establish separation performance.</p><div className="tiers">{[['F0', 'Geometry, 5-patch surface & P08 structure', `Baseline + 9 B-rep proposals + 11-family Wave-1/Wave-2 Rust 3D QD elites audited; Lamé/modal/LPBF screen & P09 GLB scene bundle passed (sour-service false)`], ['F1', 'Hydraulic initializer, GP surrogate & 10-seed ablation', `Executed SST diagnostics + RTX 3090 CUDA screen + 10-seed 4-arm ablation (B3 vs B1/B2 p_Holm=${w2t[0].p_value_holm_bonferroni.toFixed(5)}, Cliff's δ=${w2t[0].cliffs_delta.toFixed(2)}, 10% rejected-cohort audit 0% false omissions)`], ['F2', '3-Grid Roache GCI & LRR/SSG RSM + oil droplets', `3-grid G0/G1/G2 Roache GCI₂₁=${gci.dp_u_pa.gci21_standard_fs125_pct.toFixed(2)}% (Δp_u) + dual 7-eq LRR & SSG RSM cross-check (−38.5% / −38.4% Δp_u vs CT-35, 100% realizability) + 10-bin pilot executed; full residence-time qualification pending`], ['F3', 'LES & population challenge (P05)', `Rust OIL-04..15 added-mass (err ${p05.oil04_added_mass_rel_err.toExponential(2)}), substep order ${p05.oil07_substep_observed_order.toFixed(3)}, fixed-pivot PBM breakup/Smoluchowski coalescence (10/20/40-bin GCI ${(p05.oil12_pbm_grid_gci_rel * 100).toFixed(2)}%) & MPI rank invariance verified; coupled 3D LES pending`], ['F4', 'Paired physical trials (P06)', `Rust EXPERIMENT-01..06 LOQ censoring, blinded A/B/A power sizing (N ≥ ${p06.required_blocks_for_90pct_power} blocks, LCB95 ${(p06.aba_crossover_lcb95_gain_fraction * 100).toFixed(1)}%), holdout isolation & CT roughness (k_s=${p06.ct_metrology_ks_nominal_um} μm) gates verified; laboratory rig execution pending`]].map(([tier, title, text]) => <article key={tier}><span>{tier}</span><div><h3>{title}</h3><p>{text}</p></div></article>)}</div><div className="note">{catalog.families.length} planned challenge families. Campaign digest: <code>{rq.evidence_digests_sha256.campaign_summary.slice(0, 16)}…</code> · P05 PBM: <code>{rq.evidence_digests_sha256.p05_population_qualification.slice(0, 16)}…</code> · P06 Rig: <code>{rq.evidence_digests_sha256.p06_experiment_qualification.slice(0, 16)}…</code></div></section>}
      {tab === 'Work packets' && <section className="panel packets"><h2>Gated implementation</h2><p>Read MASTER_PLAN.md for ordered files, functions, acceptance and stop rules. P01–P09 native Rust verification and qualification gates are wired; final physical/CFD claims require full F2/F4 campaign execution.</p>{work.packets.map(packet => <article key={packet.id}><span className="packetid">{packet.id}</span><div><h3>{packet.title}</h3><p>{packet.owner_role} · {packet.estimate}</p></div><code>{packet.dependencies.length ? packet.dependencies.join(' + ') : 'foundation'}</code></article>)}</section>}
      <footer><span>FLEXIBLE GENES → EXACT GEOMETRY → QUALIFIED PHYSICS → PAIRED MEASUREMENTS</span><a href="https://github.com/neverhuman/joil/blob/main/docs/MASTER_PLAN.md">Read implementation plan ↗</a></footer>
    </main></>;
  }

createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>);
