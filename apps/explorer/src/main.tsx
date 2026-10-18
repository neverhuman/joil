import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { applyRevocation, mergeRefreshedManifest, isManifest, resolveIdentity, type EvidenceValue, type Manifest, type Tier } from './contracts';
import { Viewport3D } from './Viewport3D';
import './style.css';
function EvidenceTable({ values }: { values: EvidenceValue[] }) {
  if (!values.length) return <p className="placeholder">No observations are reported in this source. Software progress is available in Campaign &amp; budget.</p>;
  return <div className="tablewrap"><table><thead><tr><th>Observation</th><th>Value</th><th>Unit</th><th>Uncertainty</th><th>Tier / state</th><th>Source</th></tr></thead><tbody>
    {values.map(v => <tr key={v.id}><th scope="row">{v.label}</th><td>{v.status === 'available' && v.value !== null ? String(v.value) : <span className="placeholder">{v.status === 'failed' ? 'Failed' : v.status === 'revoked' ? 'Revoked' : 'Not reported'}</span>}</td><td>{v.unit ?? 'Not reported'}</td><td>{v.uncertainty ?? 'Not reported'}</td><td><span className={`badge ${v.status}`}>{v.tier.replaceAll('_', ' ')} · {v.status}</span></td><td>{v.source ? <a href={v.source} target="_blank" rel="noreferrer">Open source ↗</a> : 'No source attached'}</td></tr>)}
  </tbody></table></div>;
}
function App() {
  const params = useMemo(() => new URLSearchParams(location.search), []);
  const source = params.get('manifest') ?? './evidence-manifest.json';
  const requestedRevision = params.get('revision');
  const requestedSubject = params.get('subject');
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [error, setError] = useState('');
  const registryFrontier = useRef<{sequence: number; revoked_claim_ids: string[]} | null>(null);
  const [tab, setTab] = useState('Evidence');
  const [weight, setWeight] = useState([50, 50]);
  useEffect(() => { let active = true; fetch(source, { cache: 'no-store' }).then(r => { if (!r.ok) throw new Error(`Manifest request failed (${r.status})`); return r.json(); }).then(raw => { if (!isManifest(raw)) throw new Error('Manifest is malformed or uses an unsupported schema version'); if (active) setManifest(previous => mergeRefreshedManifest(previous, raw, registryFrontier.current)); }).catch(e => { if (active) setError(String(e)); }); return () => { active = false; }; }, [source]);
  useEffect(() => {
    const stream = new EventSource(`${source}${source.includes('?') ? '&' : '?'}events=1`);
    stream.addEventListener('registry', event => {
      try {
        const payload = JSON.parse((event as MessageEvent<string>).data) as { sequence?: number; revoked_claim_ids?: unknown };
        if (!Number.isSafeInteger(payload.sequence) || Number(payload.sequence) < 0 || !Array.isArray(payload.revoked_claim_ids) || !payload.revoked_claim_ids.every(x => typeof x === 'string')) throw new Error('Invalid registry event');
        const prior = registryFrontier.current;
        registryFrontier.current = {sequence: Math.max(prior?.sequence ?? 0, Number(payload.sequence)), revoked_claim_ids: [...new Set([...(prior?.revoked_claim_ids ?? []), ...payload.revoked_claim_ids])]};
        const frontier = registryFrontier.current;
        setManifest(previous => previous ? applyRevocation(previous, frontier.sequence, frontier.revoked_claim_ids) : previous);
      } catch { setError('Invalid registry event. Reload a reviewed manifest before using current evidence.'); return; }
      fetch(source, { cache: 'reload' }).then(r => { if (!r.ok) throw new Error(`Registry refresh failed (${r.status})`); return r.json(); }).then(raw => { if (isManifest(raw)) setManifest(previous => mergeRefreshedManifest(previous, raw, registryFrontier.current)); else setError('Registry refresh returned an invalid manifest. Cached data is marked stale.'); }).catch(e => setError(String(e)));
    });
    return () => stream.close();
  }, [source]);
  if (error) return <main><h1>Evidence unavailable</h1><p role="alert">{error}</p><p>No values have been substituted.</p></main>;
  if (!manifest) return <main><h1>Loading reviewed evidence</h1><p role="status">Waiting for a versioned read-only manifest.</p></main>;
  const revisionMatches = !!requestedRevision && requestedRevision === manifest.revision;
  const selected = resolveIdentity(manifest, requestedRevision, requestedSubject);
  const stale = !revisionMatches || (!!requestedSubject && !selected);
  const values = selected ? selected.values : manifest.values;
  const revoked = new Set([...(manifest.registry_event?.revoked_claim_ids ?? []), ...(manifest.offline_dossier?.revoked_claim_ids ?? [])]);
  const visible = values.map(v => revoked.has(v.id) ? { ...v, status: 'revoked' as const } : v);
  const completedPct = manifest.campaign.tasks_total && manifest.campaign.tasks_completed !== null ? Math.round(manifest.campaign.tasks_completed / manifest.campaign.tasks_total * 100) : null;
  const subjects = manifest.subjects;
  const chosenSubject = selected;
  const overlays = chosenSubject?.overlays.filter(o => o.geometry_sha256 === chosenSubject.geometry_sha256) ?? [];
  const offline = manifest.offline_dossier;
  return <>
    <header><a className="brand" href="?">JOIL <span>/ Evidence Explorer</span></a><span className="state">READ ONLY · {manifest.current ? 'CURRENT' : 'STALE'}</span></header>
    <main>
      <div className="title"><div><div className="eyebrow">DATASET REVISION {manifest.revision}</div><h1>Evidence, progress, provenance.</h1><p>Generated {manifest.generated_at}. Values are shown only with their declared source, unit, uncertainty, and evidence tier.</p></div></div>
      {stale && <div className="alert" role="alert"><strong>Identity not resolved.</strong> This URL does not identify this exact dataset revision and subject. No replacement subject has been selected. <a href={`?manifest=${encodeURIComponent(source)}&revision=${encodeURIComponent(manifest.revision)}`}>Open this revision without a subject</a>.</div>}
      {!manifest.current && <div className="alert" role="status">This manifest is stale; its observations are historical and have no current eligibility.</div>}
      <nav aria-label="Evidence explorer views">{['Evidence','Campaign & budget','Subjects & lineage','Offline dossier','Weight scratch'].map(name => <button key={name} className={tab===name?'active':''} onClick={()=>setTab(name)}>{name}</button>)}</nav>
      {tab === 'Evidence' && <><section className="panel"><h2>{selected ? `Subject ${selected.id}` : 'Registered observations'}</h2><EvidenceTable values={visible} /><p className="muted">A displayed value is descriptive evidence. Current claim eligibility must be resolved by the registered policy.</p><h3>Historical corrections</h3>{manifest.historical_corrections.length ? <ul>{manifest.historical_corrections.map(c=><li key={c.id}>{c.label}{c.source ? <> · <a href={c.source}>open source</a></> : ' · source not attached'}</li>)}</ul> : <p className="placeholder">No correction records attached.</p>}</section><Viewport3D subject={selected}/></>}
      {tab === 'Campaign & budget' && <section className="panel"><h2>Campaign progress and admitted capability</h2><div className="metrics"><div><small>Software tasks</small><strong>{completedPct === null ? 'Not reported' : `${completedPct}%`}</strong><span>{manifest.campaign.tasks_completed ?? '—'} / {manifest.campaign.tasks_total ?? '—'} completed</span></div><div><small>Attempts</small><strong>{manifest.campaign.attempts ?? 'Not reported'}</strong><span>Missing remains unknown</span></div><div><small>Spend / reserved</small><strong>{manifest.campaign.spend ?? 'Not reported'}</strong><span>Reserved: {manifest.campaign.reservations ?? 'Not reported'}</span></div><div><small>Scientific release</small><strong>{manifest.campaign.scientific_release}</strong><span>Software: {manifest.campaign.software_status}</span></div></div><dl><dt>Checkpoint</dt><dd>{manifest.campaign.checkpoint ?? 'Not reported'}</dd><dt>Control arms</dt><dd>{manifest.campaign.control_arms.length ? manifest.campaign.control_arms.join(', ') : 'None recorded'}</dd><dt>Admitted capabilities</dt><dd>{manifest.campaign.admitted_capabilities.length ? manifest.campaign.admitted_capabilities.join(', ') : 'None admitted'}</dd><dt>Blockers</dt><dd>{manifest.campaign.blockers.length ? manifest.campaign.blockers.join('; ') : 'None recorded'}</dd></dl><p className="alert">Task completion describes software progress. It does not establish physical validation or scientific release.</p></section>}
      {tab === 'Subjects & lineage' && <section className="panel"><h2>Exact subject inspection</h2><p>Selecting a subject updates both revision and subject identity in the URL. Only overlays recorded for the selected geometry may be opened.</p><label>Subject <select value={selected?.id ?? ''} onChange={e=>{const id=e.target.value; history.pushState({},'',`?manifest=${encodeURIComponent(source)}&revision=${encodeURIComponent(manifest.revision)}&subject=${encodeURIComponent(id)}`); location.reload();}}><option value="">Choose exact subject</option>{subjects.map(s=><option key={s.id} value={s.id}>{s.id} · {s.geometry_sha256.slice(0,12)}</option>)}</select></label>{chosenSubject && <><p>Geometry SHA-256: <code>{chosenSubject.geometry_sha256}</code></p><h3>Supported field overlays</h3>{overlays.length ? <ul>{overlays.map(o=><li key={o.id}>{o.field} at {o.time_s} s — <a href={o.source}>open artifact</a></li>)}</ul> : <p className="placeholder">No geometry-matched field overlays are available.</p>}<h3>Discrete recorded ancestry</h3>{chosenSubject.ancestry.length ? <ol>{chosenSubject.ancestry.map(id=><li key={id}>{id}</li>)}</ol> : <p className="placeholder">No recorded ancestry is attached.</p>}<p className="muted">No interpolated ancestors or reconstructed particle paths are generated.</p><h3>Archive points</h3><div className="tablewrap"><table><thead><tr><th>Point</th><th>X</th><th>Y</th><th>Tier</th></tr></thead><tbody>{manifest.archive.filter(p=>p.subject_id===chosenSubject.id).map(p=><tr key={p.id}><th>{p.id}</th><td>{p.x ?? 'Not reported'} {p.x===null?'':p.unit_x}</td><td>{p.y ?? 'Not reported'} {p.y===null?'':p.unit_y}</td><td>{p.tier}</td></tr>)}</tbody></table></div></>}</section>}
      {tab === 'Offline dossier' && <section className="panel"><h2>Reviewed offline dossier</h2>{offline ? <><p>Reviewed: {offline.reviewed ? 'yes' : 'no'} · revision: {offline.revision} · metadata current: {offline.current ? 'yes' : 'no'}</p><p>Source: <a href={offline.source}>open dossier</a></p><p>Revoked claims: {offline.revoked_claim_ids.length ? offline.revoked_claim_ids.join(', ') : 'None recorded'}</p>{(offline.revoked_claim_ids.length || offline.revision!==manifest.revision || !offline.current) && <p className="alert">Offline dossier is stale or contains revoked claims; affected values are ineligible.</p>}</> : <p className="placeholder">No reviewed offline dossier is attached.</p>}{manifest.registry_event && <p>Registry event {manifest.registry_event.sequence}: cache invalidated and manifest reloaded.</p>}</section>}
      {tab === 'Weight scratch' && <section className="panel"><h2>Unregistered preference scratch view</h2><p>This browser-only comparison cannot alter registered requirements, stored decisions, or scientific eligibility.</p><label>Objective A <input type="range" min="0" max="100" value={weight[0]} onChange={e=>setWeight([Number(e.target.value),100-Number(e.target.value)])}/></label><label>Objective B <input type="range" min="0" max="100" value={weight[1]} onChange={e=>setWeight([100-Number(e.target.value),Number(e.target.value)])}/></label><p>Scratch weights: {weight[0]}% / {weight[1]}%. Stored decision: unchanged.</p></section>}
      <footer><span>Read-only manifest · schema 1 · {manifest.current ? 'current metadata' : 'historical metadata'}</span><a href={source}>Open manifest</a>{manifest.source_snapshot && <a href={manifest.source_snapshot.source}>Open exact software source snapshot</a>}</footer>
    </main>
  </>;
}
createRoot(document.getElementById('root')!).render(<App />);
