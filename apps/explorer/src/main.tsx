import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import genome from '../../../baselines/ct35/genome.json';
import reference from '../../../baselines/ct35/reference.json';
import status from '../../../docs/STATUS.json';
import work from '../../../docs/WORK_PACKETS.json';
import catalog from '../../../docs/TEST_CATALOG.json';
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

function App() {
  const [tab, setTab] = useState('Geometry');
  const [full, setFull] = useState(false);
  const p = genome.parameters;
  return <><header><a className="brand" href="#">j<span>oil</span><i> / research</i></a><div className="project">EVOLUTIONARY FLUID SYSTEMS</div><span className="state">FOUNDATION · NO CFD RESULTS</span></header>
    <main><div className="eyebrow">DESIGN SUBJECT / CT-35</div><div className="title"><h1>Better separation.<br /><em>Evidence first.</em></h1><p>Explore internal geometry. Preserve interfaces.<br />Measure every claim against shared duty and uncertainty.</p></div>
      <div className="metrics"><div><small>Reference diameter</small><strong>35 <b>mm</b></strong><span>Colman–Thew reconstruction</span></div><div><small>Reference feed</small><strong>4.0 <b>m³/h</b></strong><span>Provisional operating point</span></div><div><small>Oil reject split</small><strong>2.0 <b>%</b></strong><span>Matched across comparisons</span></div><div><small>Research water quality</small><strong>30 <b>mg/L</b></strong><span>20 mg/L separate stretch duty</span></div></div>
      <nav aria-label="Research views">{['Geometry', 'Evidence', 'Work packets'].map(name => <button key={name} className={tab === name ? 'active' : ''} onClick={() => setTab(name)}>{name}</button>)}</nav>
      {tab === 'Geometry' && <div className="geometry"><section className="panel"><div className="panelhead"><div><h2>CT-35 / analytic section</h2><p>Schematic only · mm · no simulated field coloring</p></div><div className="toggle"><button className={!full ? 'selected' : ''} onClick={() => setFull(false)}>Head</button><button className={full ? 'selected' : ''} onClick={() => setFull(true)}>Full liner</button></div></div><Section full={full} /><div className="legend"><span><i className="dot gold" />Upstream design region</span><span><i className="dot cyan" />Fluid profile</span><span>Actual STEP, wall thickness and patch audit live in generated artifacts.</span></div></section><aside className="panel"><h2>Baseline contract</h2><dl><dt>Genome</dt><dd>{genome.family}</dd><dt>Chamber ID</dt><dd>70 mm</dd><dt>Oil bore</dt><dd>{(p.d2_mm * p.do_over_d2).toFixed(1)} mm</dd><dt>Twin feed area</dt><dd>2 × 120.264 mm²</dd><dt>Fluid inventory</dt><dd>1.1058 L / includes extensions</dd><dt>Nominal V/Q</dt><dd>0.995 s / not residence distribution</dd></dl><div className="note"><strong>Fit remains unresolved</strong><p>Assumed 45 mm radial envelope conflicts with inlet ports. Actual vessel drawing required before fit qualification.</p></div></aside></div>}
      {tab === 'Evidence' && <section className="panel evidence"><h2>Confidence must be earned</h2><p>Current state: {status.phase.replaceAll('_', ' ')}. Arithmetic and CAD checks cannot establish separation performance.</p><div className="tiers">{[['F0', 'Geometry & manufacture', 'Baseline CAD audit; full manufacturing gate pending'], ['F1', 'Cheap hydraulic screen', 'Planned / no solver run'], ['F2', 'Transient RSM + oil droplets', 'Planned / force and dispersion qualification required'], ['F3', 'LES & population challenge', 'Planned / independent model checks'], ['F4', 'Paired physical trials', 'Pending laboratory evidence']].map(([tier, title, text]) => <article key={tier}><span>{tier}</span><div><h3>{title}</h3><p>{text}</p></div></article>)}</div><div className="note">{catalog.families.length} planned challenge families. Executed checks and artifact hashes belong in receipts; planned families are not passed-test counts.</div></section>}
      {tab === 'Work packets' && <section className="panel packets"><h2>Gated implementation</h2><p>Read MASTER_PLAN.md for ordered files, functions, acceptance and stop rules. These packets have not been dispatched.</p>{work.packets.map(packet => <article key={packet.id}><span className="packetid">{packet.id}</span><div><h3>{packet.title}</h3><p>{packet.owner_role} · {packet.estimate}</p></div><code>{packet.dependencies.length ? packet.dependencies.join(' + ') : 'foundation'}</code></article>)}</section>}
      <footer><span>FLEXIBLE GENES → EXACT GEOMETRY → QUALIFIED PHYSICS → PAIRED MEASUREMENTS</span><a href="https://github.com/neverhuman/joil/blob/main/docs/MASTER_PLAN.md">Read implementation plan ↗</a></footer>
    </main></>;
}

createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>);
