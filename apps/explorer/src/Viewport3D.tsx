import React, { useEffect, useRef, useState } from 'react';
import bundleData from '../../../docs/results/interactive-3d-archetypes-v1.json';

interface DropletTrajectory3D {
  label: string;
  diameter_um: number;
  /** Authored display selection (diameter >= screen d50), not an observed capture result. */
  captured_to_overflow: boolean;
  points_mm: number[][];
  /** Absent in historical bundles, which hold the same authored curves (V5-01-04). */
  provenance?: string;
}

const SCHEMATIC_PROVENANCE = 'SCHEMATIC_AUTHORED_CURVE_NOT_SIMULATED_OR_MEASURED';

/** Missing provenance reads as schematic; only an explicit non-schematic value would differ. */
function pathProvenance(t: DropletTrajectory3D): string {
  return t.provenance ?? SCHEMATIC_PROVENANCE;
}

interface Candidate3D {
  id: string;
  topology_class: string;
  genome_sha256: string;
  generation_born: number;
  island_origin: number;
  branches_per_port: number;
  axial_drop_mm: number;
  radial_slot_factor: number;
  skimmer_length_mm: number;
  skimmer_flutes: number;
  chamber_mode_count: number;
  nominal_cout_mg_l: number;
  cout_p05_mg_l: number;
  cout_p50_mg_l: number;
  cout_p95_mg_l: number;
  dp_u_pa: number;
  pdr: number;
  d50_um: number;
  grade_20um_p05: number;
  max_entry_shear_s_inv: number;
  short_circuit_fraction: number;
  p08_yield_safety_factor: number;
  p08_max_von_mises_mpa: number;
  p08_vane_modal_freq_ratio: number;
  p08_lpbf_overhang_deg: number;
  min_wall_clearance_mm: number;
  grade_curve_bins: number[][];
  outer_wall_rings_mm: number[][][];
  skimmer_rings_mm: number[][][];
  inlet_jets_3d_mm: number[][][];
  helical_ribbon_3d_mm: number[][];
  droplet_trajectories_3d_mm: DropletTrajectory3D[];
}

interface ArchivePoint {
  id: string;
  topology_class: string;
  cell_key: string;
  cout_p95_mg_l: number;
  dp_u_pa: number;
  d50_um: number;
  grade_20um_p05: number;
  max_entry_shear_s_inv: number;
  swirl_flux_index: number;
  core_shield_ratio: number;
  is_pareto_rank0: boolean;
}

const FAMILY_COLORS: Record<string, string> = {
  'baseline-ct35': '#90a4ae',
  'helical-pitched-ribbon': '#d4b06a',
  'contoured-s-cone-skimmer': '#72c1c1',
  'bifurcated-quad-manifold': '#7be396',
  'trifurcated-6jet-vortex-head': '#38bdf8',
  'spiral-rifled-vortex-head': '#f08c6c',
  'multi-stage-harmonic-chamber': '#b993f5',
  'hybrid-omni-exotic': '#5de0e6',
  'cylindrical-gyroid-coalescer': '#2dd4bf',
  'schwarz-p-rectifier-head': '#fb7185',
  'octa-branch-helical-manifold': '#a3e635',
  'counter-fluted-recovery-vortex': '#facc15',
};

export function Viewport3D() {
  const candidates = bundleData.candidates_3d as Candidate3D[];
  const archive = bundleData.archive_points as ArchivePoint[];
  const baseline = candidates[0];

  const [selectedIdx, setSelectedIdx] = useState<number>(candidates.length > 1 ? 1 : 0);
  const [cutaway, setCutaway] = useState<boolean>(true);
  const [showDroplets, setShowDroplets] = useState<boolean>(true);
  const [autoSpin, setAutoSpin] = useState<boolean>(true);
  const [zoom, setZoom] = useState<number>(1.08);

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rotRef = useRef<{ yaw: number; pitch: number; phase: number }>({
    yaw: -0.55,
    pitch: 0.34,
    phase: 0,
  });
  const dragRef = useRef<{ active: boolean; lastX: number; lastY: number }>({
    active: false,
    lastX: 0,
    lastY: 0,
  });

  const cand = candidates[selectedIdx] ?? baseline;
  const pathsSchematic = cand.droplet_trajectories_3d_mm.every((t) =>
    pathProvenance(t).startsWith('SCHEMATIC'),
  );
  const pathsLabel = pathsSchematic
    ? 'SCHEMATIC authored paths (closed-form curves; not simulated or measured)'
    : `Paths with mixed provenance: ${[...new Set(cand.droplet_trajectories_3d_mm.map(pathProvenance))].join(', ')}`;

  useEffect(() => {
    let animId = 0;
    const render = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      if (autoSpin && !dragRef.current.active) {
        rotRef.current.yaw += 0.0065;
      }
      rotRef.current.phase = (rotRef.current.phase + 0.022) % 1.0;

      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      // Background subtle radial glow
      const grad = ctx.createRadialGradient(w * 0.48, h * 0.5, 20, w * 0.48, h * 0.5, w * 0.6);
      grad.addColorStop(0, '#182c3a');
      grad.addColorStop(1, '#0e1820');
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, w, h);

      const yaw = rotRef.current.yaw;
      const pitch = rotRef.current.pitch;
      const cy = Math.cos(yaw);
      const sy = Math.sin(yaw);
      const cp = Math.cos(pitch);
      const sp = Math.sin(pitch);

      // Center axial coordinate z around z_mid = 82 mm; map +z horizontally, x/y radially
      const scale = 3.25 * zoom;
      const project = (pt: number[]): [number, number, number] => {
        const x = pt[0];
        const y = pt[1];
        const z = pt[2] - 82.0;
        // Rotate around vertical axis (yaw) and tilt (pitch)
        const rx = z * cy - x * sy;
        const rz = z * sy + x * cy;
        const ry = y * cp - rz * sp;
        const depth = y * sp + rz * cp;
        const persp = 560.0 / (560.0 - depth * 0.75);
        return [w * 0.5 + rx * scale * persp, h * 0.52 - ry * scale * persp, depth];
      };

      // Draw centerline axis (z from -25 to 190 mm)
      const pAxis0 = project([0, 0, -25]);
      const pAxis1 = project([0, 0, 192]);
      ctx.strokeStyle = '#405666';
      ctx.lineWidth = 1;
      ctx.setLineDash([5, 5]);
      ctx.beginPath();
      ctx.moveTo(pAxis0[0], pAxis0[1]);
      ctx.lineTo(pAxis1[0], pAxis1[1]);
      ctx.stroke();
      ctx.setLineDash([]);

      // 1. Outer chamber & reducing cone 3D rings
      const rings = cand.outer_wall_rings_mm;
      for (let iz = 0; iz < rings.length; iz++) {
        const ring = rings[iz];
        const n = ring.length;
        ctx.strokeStyle = iz === 0 || iz === rings.length - 1 ? '#87c5c5' : '#2e5d6e';
        ctx.lineWidth = iz === 0 || iz === rings.length - 1 ? 1.5 : 1.0;
        ctx.beginPath();
        let started = false;
        for (let it = 0; it <= n; it++) {
          const pt = ring[it % n];
          const [px, py, depth] = project(pt);
          if (cutaway && depth > 10.0) {
            started = false;
            continue;
          }
          if (!started) {
            ctx.moveTo(px, py);
            started = true;
          } else {
            ctx.lineTo(px, py);
          }
        }
        ctx.stroke();
      }

      // Longitudinal ribs connecting outer wall rings
      if (rings.length > 1) {
        const nTheta = rings[0].length;
        for (let it = 0; it < nTheta; it += 2) {
          ctx.strokeStyle = '#254a59';
          ctx.lineWidth = 0.85;
          ctx.beginPath();
          let started = false;
          for (let iz = 0; iz < rings.length; iz++) {
            const [px, py, depth] = project(rings[iz][it]);
            if (cutaway && depth > 10.0) {
              started = false;
              continue;
            }
            if (!started) {
              ctx.moveTo(px, py);
              started = true;
            } else {
              ctx.lineTo(px, py);
            }
          }
          ctx.stroke();
        }
      }

      // 2. Internal helical swirl ribbon
      if (cand.helical_ribbon_3d_mm.length > 1) {
        ctx.strokeStyle = '#7be396';
        ctx.lineWidth = 2.0;
        ctx.beginPath();
        cand.helical_ribbon_3d_mm.forEach((pt, i) => {
          const [px, py] = project(pt);
          if (i === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        });
        ctx.stroke();
      }

      // 3. Contoured & fluted vortex-finder skimmer
      cand.skimmer_rings_mm.forEach((ring, iz) => {
        ctx.strokeStyle = iz === 0 ? '#f08c6c' : '#d4b06a';
        ctx.lineWidth = 1.65;
        ctx.beginPath();
        ring.forEach((pt, it) => {
          const [px, py] = project(pt);
          if (it === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        });
        const [px0, py0] = project(ring[0]);
        ctx.lineTo(px0, py0);
        ctx.stroke();
      });

      // 4. Multi-branch 3D tangential entry jets
      cand.inlet_jets_3d_mm.forEach((jet) => {
        ctx.strokeStyle = '#5de0e6';
        ctx.lineWidth = 2.2;
        ctx.beginPath();
        jet.forEach((pt, i) => {
          const [px, py] = project(pt);
          if (i === 0) ctx.moveTo(px, py);
          else ctx.lineTo(px, py);
        });
        ctx.stroke();
      });

      // 5. SCHEMATIC authored droplet and carrier paths (no particle is integrated)
      if (showDroplets) {
        const trajColors = ['#ffd166', '#ff9f43', '#f368e0', '#48dbfb'];
        cand.droplet_trajectories_3d_mm.forEach((traj, idx) => {
          const col = trajColors[idx % trajColors.length];
          ctx.strokeStyle = col;
          ctx.lineWidth = traj.diameter_um >= 20 ? 2.1 : 1.5;
          ctx.beginPath();
          traj.points_mm.forEach((pt, i) => {
            const [px, py] = project(pt);
            if (i === 0) ctx.moveTo(px, py);
            else ctx.lineTo(px, py);
          });
          ctx.stroke();

          // Animated schematic marker along the authored path
          const pCount = traj.points_mm.length;
          if (pCount > 1) {
            const fIdx = ((rotRef.current.phase + idx * 0.23) % 1.0) * (pCount - 1);
            const i0 = Math.floor(fIdx);
            const i1 = Math.min(pCount - 1, i0 + 1);
            const alpha = fIdx - i0;
            const p0 = traj.points_mm[i0];
            const p1 = traj.points_mm[i1];
            const interp = [
              p0[0] + (p1[0] - p0[0]) * alpha,
              p0[1] + (p1[1] - p0[1]) * alpha,
              p0[2] + (p1[2] - p0[2]) * alpha,
            ];
            const [mx, my] = project(interp);
            ctx.fillStyle = col;
            ctx.beginPath();
            ctx.arc(mx, my, traj.diameter_um >= 20 ? 4.2 : 3.2, 0, Math.PI * 2);
            ctx.fill();
          }
        });
        ctx.fillStyle = '#ffd166';
        ctx.font = '12px ui-monospace, monospace';
        ctx.fillText(pathsSchematic ? 'SCHEMATIC paths: authored, not simulated' : 'Paths: mixed provenance', 14, 22);
      }

      // Labels in 3D space
      ctx.fillStyle = '#b4c7d2';
      ctx.font = '11px ui-monospace, monospace';
      const [ox, oy] = project([0, 0, -22.8]);
      ctx.fillText('Oil reject (-22.8 mm)', ox - 45, oy - 12);
      const [cx, cy2] = project([0, 0, 186.75]);
      ctx.fillText('Collar z=186.75 mm', cx - 50, cy2 + 22);

      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animId);
  }, [cand, cutaway, showDroplets, autoSpin, zoom, pathsSchematic]);

  const onMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    dragRef.current = { active: true, lastX: e.clientX, lastY: e.clientY };
  };
  const onMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!dragRef.current.active) return;
    const dx = e.clientX - dragRef.current.lastX;
    const dy = e.clientY - dragRef.current.lastY;
    dragRef.current.lastX = e.clientX;
    dragRef.current.lastY = e.clientY;
    rotRef.current.yaw += dx * 0.008;
    rotRef.current.pitch = Math.max(-1.1, Math.min(1.1, rotRef.current.pitch + dy * 0.008));
  };
  const onMouseUp = () => {
    dragRef.current.active = false;
  };

  const coutDeltaPct =
    ((cand.cout_p95_mg_l - baseline.cout_p95_mg_l) / baseline.cout_p95_mg_l) * 100;
  const shearDeltaPct =
    ((cand.max_entry_shear_s_inv - baseline.max_entry_shear_s_inv) /
      baseline.max_entry_shear_s_inv) *
    100;

  return (
    <div className="viewport-suite">
      <div className="geometry">
        <section className="panel">
          <div className="panelhead">
            <div>
              <h2>
                Interactive 3D Hydrocyclone Geometry &amp; Schematic Oil Paths (<code>{cand.id}</code>)
              </h2>
              <p>
                True-scale millimetre B-rep rings · drag to orbit · 10/20/40 μm oil and carrier
                paths: {pathsLabel}
              </p>
            </div>
            <div className="toggle">
              <button className={cutaway ? 'selected' : ''} onClick={() => setCutaway(!cutaway)}>
                Cutaway
              </button>
              <button
                className={showDroplets ? 'selected' : ''}
                onClick={() => setShowDroplets(!showDroplets)}
              >
                Schematic paths
              </button>
              <button className={autoSpin ? 'selected' : ''} onClick={() => setAutoSpin(!autoSpin)}>
                Orbit
              </button>
              <button onClick={() => setZoom((z) => Math.min(1.6, z + 0.14))}>+</button>
              <button onClick={() => setZoom((z) => Math.max(0.7, z - 0.14))}>−</button>
            </div>
          </div>

          <label className="runpicker">
            3D Archetype / Pareto Elite{' '}
            <select
              value={selectedIdx}
              onChange={(e) => setSelectedIdx(Number(e.target.value))}
            >
              {candidates.map((c, i) => (
                <option key={c.id} value={i}>
                  {c.id} [{c.topology_class}] — Cout P95: {c.cout_p95_mg_l.toFixed(2)} mg/L · d50:{' '}
                  {c.d50_um.toFixed(2)} μm · Δp_u: {(c.dp_u_pa / 1000).toFixed(2)} kPa
                </option>
              ))}
            </select>
          </label>

          <canvas
            ref={canvasRef}
            width={760}
            height={420}
            className="canvas3d"
            onMouseDown={onMouseDown}
            onMouseMove={onMouseMove}
            onMouseUp={onMouseUp}
            onMouseLeave={onMouseUp}
          />

          <div className="legend">
            <span>
              <i className="dot cyan" />
              Twin {cand.branches_per_port}-branch tangential entry jets
            </span>
            <span>
              <i className="dot gold" />
              Contoured skimmer ({cand.skimmer_length_mm.toFixed(1)} mm, {cand.skimmer_flutes}{' '}
              flutes)
            </span>
            <span>
              <i className="dot" style={{ background: '#ffd166' }} />
              SCHEMATIC 40 μm &amp; 20 μm oil paths toward the reject (authored; shown captured
              when d ≥ screen d50)
            </span>
            <span>
              <i className="dot" style={{ background: '#48dbfb' }} />
              SCHEMATIC treated-water outer path (authored)
            </span>
          </div>
        </section>

        <aside className="panel">
          <h2>3D Phenotype &amp; P08 Scorecard</h2>
          <dl>
            <dt>Topology family</dt>
            <dd>{cand.topology_class}</dd>
            <dt>Screen Cout (nom / P95)</dt>
            <dd>
              {cand.nominal_cout_mg_l.toFixed(2)} / {cand.cout_p95_mg_l.toFixed(2)} mg/L (
              {coutDeltaPct <= 0 ? `${coutDeltaPct.toFixed(1)}%` : `+${coutDeltaPct.toFixed(1)}%`}{' '}
              vs CT-35)
            </dd>
            <dt>Cut size d50 / G(20 μm) P05</dt>
            <dd>
              {cand.d50_um.toFixed(2)} μm · {(cand.grade_20um_p05 * 100).toFixed(1)}%
            </dd>
            <dt>Hydraulic Δp_u / PDR</dt>
            <dd>
              {(cand.dp_u_pa / 1000).toFixed(2)} kPa · PDR {cand.pdr.toFixed(2)}
            </dd>
            <dt>Entry shear rate G_max</dt>
            <dd>
              {cand.max_entry_shear_s_inv.toFixed(0)} s⁻¹ ({shearDeltaPct.toFixed(1)}% vs CT-35)
            </dd>
            <dt>Inlet branches / axial drop</dt>
            <dd>
              2 × {cand.branches_per_port} ports · Δz = {cand.axial_drop_mm.toFixed(2)} mm
            </dd>
            <dt>P08 Ti-6Al-4V yield SF / σ_vm</dt>
            <dd>
              {cand.p08_yield_safety_factor.toFixed(2)}× ({cand.p08_max_von_mises_mpa.toFixed(1)}{' '}
              MPa)
            </dd>
            <dt>Vane modal ratio / LPBF angle</dt>
            <dd>
              {cand.p08_vane_modal_freq_ratio.toFixed(1)}× · {cand.p08_lpbf_overhang_deg.toFixed(1)}°
            </dd>
            <dt>Genome SHA-256</dt>
            <dd className="digest">{cand.genome_sha256.slice(0, 24)}…</dd>
          </dl>
        </aside>
      </div>

      <div className="geometry" style={{ marginTop: '20px' }}>
        <section className="panel">
          <div className="panelhead">
            <div>
              <h2>Pareto Front &amp; 4D MAP-Elites Archive ({archive.length} QD Elites)</h2>
              <p>
                Click any highlighted Rank-0 Pareto elite to inspect its 3D geometry &amp; schematic
                paths above
              </p>
            </div>
          </div>
          <svg
            viewBox="0 0 640 270"
            role="img"
            aria-label="Pareto front scatter of pressure drop vs P95 outlet oil concentration"
          >
            <rect x="0" y="0" width="640" height="270" fill="#101b24" />
            {/* 30 mg/L research threshold line */}
            <line
              x1="55"
              y1={230 - ((30.0 - 15.0) / 16.0) * 195}
              x2="615"
              y2={230 - ((30.0 - 15.0) / 16.0) * 195}
              stroke="#d4b06a"
              strokeDasharray="5 4"
            />
            <text x="455" y={224 - ((30.0 - 15.0) / 16.0) * 195} fill="#d4b06a">
              30 mg/L target threshold
            </text>
            <line x1="55" y1="230" x2="615" y2="230" stroke="#455a68" />
            <line x1="55" y1="25" x2="55" y2="230" stroke="#455a68" />
            <text x="245" y="258">
              Underflow Pressure Drop Δp_u (kPa) →
            </text>
            <text x="12" y="20">
              Cout P95 (mg/L)
            </text>
            {archive.map((pt) => {
              const x = 55 + ((pt.dp_u_pa / 1000 - 25.0) / 6.8) * 550;
              const y = 230 - ((pt.cout_p95_mg_l - 15.0) / 16.0) * 195;
              const col = FAMILY_COLORS[pt.topology_class] ?? '#8ba0ae';
              const matchIdx = candidates.findIndex((c) => c.id === pt.id);
              const isSelected = cand.id === pt.id;
              return (
                <circle
                  key={pt.cell_key}
                  cx={Math.max(58, Math.min(610, x))}
                  cy={Math.max(28, Math.min(226, y))}
                  r={isSelected ? 6.5 : pt.is_pareto_rank0 ? 4.8 : 3.0}
                  fill={col}
                  opacity={pt.is_pareto_rank0 ? 0.95 : 0.45}
                  stroke={isSelected ? '#ffffff' : pt.is_pareto_rank0 ? '#101b24' : 'none'}
                  strokeWidth={isSelected ? 2.0 : 1.0}
                  style={{ cursor: matchIdx >= 0 ? 'pointer' : 'default' }}
                  onClick={() => {
                    if (matchIdx >= 0) setSelectedIdx(matchIdx);
                  }}
                >
                  <title>
                    {pt.id} ({pt.topology_class}) — Cout P95: {pt.cout_p95_mg_l} mg/L, Δp_u:{' '}
                    {(pt.dp_u_pa / 1000).toFixed(2)} kPa, d50: {pt.d50_um} μm
                  </title>
                </circle>
              );
            })}
          </svg>
        </section>

        <aside className="panel">
          <h2>10-Bin Grade Efficiency G(d)</h2>
          <p>
            <code>{cand.id}</code> (gold) vs. <code>CT-35</code> baseline (dashed cyan)
          </p>
          <svg
            viewBox="0 0 320 235"
            role="img"
            aria-label="10-bin droplet grade efficiency curve comparison"
          >
            <rect x="0" y="0" width="320" height="235" fill="#101b24" />
            <line x1="36" y1="198" x2="304" y2="198" stroke="#455a68" />
            <line x1="36" y1="20" x2="36" y2="198" stroke="#455a68" />
            {/* 50% cut-size line */}
            <line
              x1="36"
              y1={198 - 0.5 * 170}
              x2="304"
              y2={198 - 0.5 * 170}
              stroke="#344856"
              strokeDasharray="3 3"
            />
            <text x="245" y={193 - 0.5 * 170}>
              d50 (50%)
            </text>
            {/* Baseline curve */}
            <polyline
              fill="none"
              stroke="#72c1c1"
              strokeWidth="1.6"
              strokeDasharray="4 3"
              points={baseline.grade_curve_bins
                .map(([d, g]) => {
                  const x = 36 + (Math.log10(d / 3.0) / Math.log10(100.0 / 3.0)) * 262;
                  const y = 198 - g * 170;
                  return `${x.toFixed(1)},${y.toFixed(1)}`;
                })
                .join(' ')}
            />
            {/* Selected candidate curve */}
            <polyline
              fill="none"
              stroke="#d4b06a"
              strokeWidth="2.4"
              points={cand.grade_curve_bins
                .map(([d, g]) => {
                  const x = 36 + (Math.log10(d / 3.0) / Math.log10(100.0 / 3.0)) * 262;
                  const y = 198 - g * 170;
                  return `${x.toFixed(1)},${y.toFixed(1)}`;
                })
                .join(' ')}
            />
            <text x="90" y="222">
              Droplet diameter d (3–100 μm, log)
            </text>
          </svg>
        </aside>
      </div>
    </div>
  );
}
