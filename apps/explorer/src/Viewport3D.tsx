import type { Subject } from './contracts';

export function Viewport3D({ subject }: { subject?: Subject }) {
  if (!subject) return <section className="panel"><h2>Geometry view</h2><p className="placeholder">Choose a subject using its exact dataset revision before inspecting geometry.</p></section>;
  const supported = subject.overlays.filter(overlay => overlay.geometry_sha256 === subject.geometry_sha256);
  return <section className="panel" aria-labelledby="geometry-heading">
    <h2 id="geometry-heading">Geometry and artifact-supported overlays</h2>
    <p>Subject <code>{subject.id}</code> · geometry SHA-256 <code>{subject.geometry_sha256}</code></p>
    {subject.geometry_source ? <p><a href={subject.geometry_source}>Open source geometry artifact</a></p> : <p className="placeholder">No geometry artifact attached; no schematic or reconstructed shape is shown.</p>}
    <h3>Matching overlays</h3>
    {supported.length ? <ul>{supported.map(o => <li key={o.id}>{o.field} · {o.time_s} s · <a href={o.source}>open source artifact</a></li>)}</ul> : <p className="placeholder">No field overlays match this exact geometry.</p>}
  </section>;
}
