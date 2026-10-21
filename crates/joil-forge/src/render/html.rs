//! Self-contained HTML run dashboard with an embedded WebGL2 cartridge viewer.

use crate::cad::mesh::Mesh;
use crate::cad::CadReport;
use crate::checkpoint::Checkpoint;
use crate::evolve::Individual;
use crate::score::Score;
use std::fmt::Write;

pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

/// Quantised meshes for the viewer: uint16 positions in the render frame, owner bytes,
/// uint32 indices.
pub struct MeshPayload {
    pub json: String,
}

fn pack(m: &Mesh, length: f64, lo: [f32; 3], hi: [f32; 3]) -> serde_json::Value {
    let mut pos = Vec::with_capacity(m.v.len() * 6);
    let mut own = Vec::with_capacity(m.v.len());
    for (i, v) in m.v.iter().enumerate() {
        let p = [v[2] - 0.5 * length as f32, -v[0], -v[1]];
        for k in 0..3 {
            let t = ((p[k] - lo[k]) / (hi[k] - lo[k]).max(1e-6)).clamp(0.0, 1.0);
            pos.extend_from_slice(&((t * 65535.0).round() as u16).to_le_bytes());
        }
        let o = m.owner.get(i).copied().unwrap_or(u16::MAX);
        own.push(if o == u16::MAX { 255 } else { (o % 255) as u8 });
    }
    let idx: Vec<u8> =
        m.f.iter()
            .flat_map(|t| t.iter().flat_map(|i| i.to_le_bytes()))
            .collect();
    serde_json::json!({"pos": base64(&pos), "own": base64(&own), "idx": base64(&idx), "n": m.v.len(), "m": m.f.len()})
}

impl MeshPayload {
    pub fn new(fluid: &Mesh, part: &Mesh, length: f64) -> Self {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in fluid.v.iter().chain(part.v.iter()) {
            let p = [v[2] - 0.5 * length as f32, -v[0], -v[1]];
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let json = serde_json::json!({"lo": lo, "hi": hi, "fluid": pack(fluid, length, lo, hi), "part": pack(part, length, lo, hi)}).to_string();
        Self { json }
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn strip_xml_decl(svg: &str) -> &str {
    svg
}

fn tile(label: &str, value: String, unit: &str, note: &str) -> String {
    format!("<div class='tile'><div class='lab'>{label}</div><div class='val'>{value}<span class='u'>{unit}</span></div><div class='note'>{note}</div></div>")
}

fn fmt_opt(v: Option<f64>, digits: usize) -> String {
    v.map(|x| format!("{x:.digits$}"))
        .unwrap_or_else(|| "–".into())
}

#[allow(clippy::too_many_arguments)]
pub fn dashboard(
    cp: &Checkpoint,
    best: &Individual,
    score: &Score,
    cad: &CadReport,
    mesh: &MeshPayload,
    pfd_svg: &str,
    grade_svg: &str,
    evo_svg: &str,
    stage_svgs: &[String],
    gallery: &[(String, String, Individual)],
) -> String {
    let c = &cp.campaign;
    let st = &cp.state;
    let status = if score.feasible {
        (
            "ok",
            "Compliant in screen: reference feed, every closure sample",
        )
    } else {
        ("warn", "Not yet compliant in screen")
    };
    let mut h = String::new();
    let _ = write!(h, "<title>{} Forge Run</title>\n<meta name=\"claude:viewport-fit-sides\" content=\"cover\">\n", esc(&c.name));
    h.push_str(CSS);
    // Figures follow the viewer's explicit theme choice as well as the OS setting.
    let _ = write!(
        h,
        "<style>{}{}</style>",
        super::svg::theme_rules(":root[data-theme=\"dark\"] .viz", true),
        super::svg::theme_rules(":root[data-theme=\"light\"] .viz", false)
    );
    let _ = write!(
        h,
        "<header class='top'><div class='wrap'><div class='eyebrow'>joil forge · evolutionary cartridge search</div><h1>{}</h1><p class='sub'>Generation {} · {} evaluations · {} islands × {} · {} archive niches ({} feasible) · <span class='pill {}'>{}</span></p></div></header>",
        esc(&c.name),
        st.generation,
        st.evaluations,
        c.islands,
        c.population,
        st.archive.cells.len(),
        st.archive.feasible(),
        status.0,
        status.1
    );
    h.push_str("<main class='wrap'>");
    h.push_str("<section class='tiles'>");
    for (d, duty) in c.duties.iter().enumerate() {
        let cout = score.duty_cout_mg_l.get(d).copied().flatten();
        let note = match cout {
            Some(v) => format!(
                "from {:.0} mg/L ({:.1}% removed), worst closure at the rated flow",
                duty.feed.cin_mg_l,
                100.0 * (1.0 - v / duty.feed.cin_mg_l)
            ),
            None => format!("from {:.0} mg/L", duty.feed.cin_mg_l),
        };
        h.push_str(&tile(
            &format!("Outlet oil: {}", duty.name),
            fmt_opt(cout, 1),
            "mg/L",
            &note,
        ));
    }
    h.push_str(&tile(
        "Conservative treated capacity",
        format!("{:.2}", score.q_treated_m3_h),
        "m³/h",
        "minimum over closure samples, at the pressure budget",
    ));
    h.push_str(&tile(
        "Pressure used",
        fmt_opt(score.feed_pressure_kpa, 0),
        "kPa",
        &format!(
            "feed plus pump work; budget {:.0} kPa",
            c.limits.pressure_budget_pa / 1e3
        ),
    ));
    h.push_str(&tile(
        "Specific energy",
        fmt_opt(score.specific_energy_kj_m3, 0),
        "kJ/m³",
        "per m³ treated, pumps included",
    ));
    h.push_str(&tile(
        "Part height",
        format!("{:.0}", cad.length_mm),
        "mm",
        &format!(
            "build height {:.0} mm; Ø{:.0} mm footprint",
            c.envelope.length_mm,
            2.0 * c.envelope.radius_mm
        ),
    ));
    h.push_str(&tile(
        "Printed mass",
        format!("{:.2}", cad.printed_mass_kg),
        "kg",
        &format!("Ti-6Al-4V shell, {:.1} mm walls", cad.wall_mm),
    ));
    h.push_str(&tile(
        "Reject fraction",
        fmt_opt(score.reject_fraction.map(|r| 100.0 * r), 2),
        "%",
        &format!("budget {:.1}%", 100.0 * c.limits.reject_budget),
    ));
    h.push_str("</section>");
    h.push_str(
        "<section class='grid2'><div class='panel viewer'><div class='ph'><h2>Best design, in 3D</h2><div class='seg' role='group' aria-label='View mode'>\
<button id='m-cut' class='on' type='button'>Cutaway</button><button id='m-x' type='button'>X-ray</button><button id='m-f' type='button'>Fluid only</button></div></div>\
<canvas id='gl' aria-label='Interactive 3D view of the cartridge: drag to orbit, scroll or pinch to zoom'></canvas>\
<div class='hint'>Drag to orbit · scroll or pinch to zoom · colours mark process units; grey channels connect them.</div></div>",
    );
    let _ = write!(
        h,
        "<div class='panel'><h2>Process network</h2><div class='scroll'>{}</div></div></section>",
        strip_xml_decl(pfd_svg)
    );
    let _ = write!(
        h,
        "<section class='grid2'><div class='panel'><h2>Search progress</h2><div class='scroll'>{}</div></div><div class='panel'><h2>Separation by stage</h2><div class='scroll'>{}</div></div></section>",
        strip_xml_decl(evo_svg),
        strip_xml_decl(grade_svg)
    );
    if !stage_svgs.is_empty() {
        h.push_str("<section class='panel'><h2>What each stage receives</h2><p class='lede'>Oil drop sizes arriving at every unit of the best design on each duty, at its rated flow. Downstream stages see less oil and finer drops, which is why a chain can use stages unlike the first.</p><div class='grid2 inner'>");
        for svg in stage_svgs {
            let _ = write!(h, "<div class='scroll'>{}</div>", strip_xml_decl(svg));
        }
        h.push_str("</div></section>");
    }
    if !gallery.is_empty() {
        h.push_str("<section class='panel'><h2>Archive: best design in each niche</h2><p class='lede'>Each niche groups designs by measured hardware: total cyclone cells (c), series depth (d), recycles (r), auxiliaries (x) and stage taper (s: 0 alike, 1 later stages smaller, 2 larger). The archive keeps unfamiliar topologies alive instead of letting one family take over; it never adds to fitness.</p><div class='zoo'>");
        for (uri, svg, ind) in gallery {
            let s = &ind.score;
            let _ = write!(
                h,
                "<figure class='card'><img src='{uri}' alt='X-ray render of the best design in niche {}'><figcaption><b>{}</b> · {} · q {:.2} m³/h · outlet {} mg/L · {} unit{}<details><summary>Process network</summary><div class='scroll'>{}</div></details></figcaption></figure>",
                esc(&ind.behavior.as_ref().map(crate::archive::niche).unwrap_or_default()),
                esc(&ind.behavior.as_ref().map(crate::archive::niche).unwrap_or_default()),
                if s.feasible { "compliant" } else { "infeasible" },
                s.q_treated_m3_h,
                s.duty_cout_mg_l
                    .iter()
                    .map(|v| fmt_opt(*v, 1))
                    .collect::<Vec<_>>()
                    .join(" / "),
                s.units,
                if s.units == 1 { "" } else { "s" },
                svg
            );
        }
        h.push_str("</div></section>");
    }
    let _ = write!(
        h,
        "<section class='panel ev'><h2>Evidence and limits</h2><dl>\
<dt>Evidence class</dt><dd><code>{}</code>. Every number on this page is a reduced-order screen result. Nothing here is CFD, a measurement, or physical validation.</dd>\
<dt>Model</dt><dd><code>{}</code>, {} closure samples (nominal + Latin hypercube) with common random numbers; feasibility requires compliance for every sample.</dd>\
<dt>Run identity</dt><dd>campaign <code>{}</code><br>best genome <code>{}</code></dd>\
<dt>Mesh audit</dt><dd>fluid: closed {} · {} components · genus {:?} · {:.0} cm³ (analytic estimate differs by {:+.1}%)<br>printed part: closed {} · {} components · {:.0} cm³ · voxel {:.2} mm</dd>\
<dt>Print audit</dt><dd>printed standing on its head; part {:.0} mm of a {:.0} mm build height · {} support posts · {} teardrop channel runs · {:.0} mm² downward-facing internal surface, {:.0} mm² of it beyond the {:.0} mm bridge from any support (screen, not a print simulation)</dd>\
<dt>Duties</dt><dd>{}</dd>\
<dt>Open gates</dt><dd>Independent CFD of the evolved geometry, droplet-resolved oil transport, structural and print qualification, and physical rig tests have not been done. The closure ranges, the CT-35 anchor and the scoring weights are declared assumptions.</dd></dl></section>",
        joil_forge_judges::EVIDENCE_CLASS,
        joil_forge_judges::MODEL_VERSION,
        c.closure_samples + 1,
        cp.manifest.campaign_sha256,
        best.hash,
        cad.fluid.closed,
        cad.fluid.components,
        cad.fluid.genus,
        cad.fluid.volume_mm3 / 1e3,
        100.0 * cad.fluid_volume_rel_diff,
        cad.part.closed,
        cad.part.components,
        cad.part.volume_mm3 / 1e3,
        cad.voxel_mm,
        cad.print.part_height_mm,
        cad.print.build_height_mm,
        cad.print.support_posts,
        cad.print.teardrop_runs,
        cad.print.ceiling_area_mm2,
        cad.print.unsupported_ceiling_area_mm2,
        cad.print.bridge_mm,
        c.duties
            .iter()
            .map(|d| format!(
                "{}: water {:.1} kg/m³, oil {:.0} kg/m³, {:.0} mg/L, median {:.0} µm (weight {})",
                esc(&d.name),
                d.fluids.rho_water_kg_m3,
                d.fluids.rho_oil_kg_m3,
                d.feed.cin_mg_l,
                d.feed.dsd.volume_median_um,
                d.weight
            ))
            .collect::<Vec<_>>()
            .join("<br>")
    );
    h.push_str("</main>");
    let _ = write!(
        h,
        "<script id='mesh' type='application/json'>{}</script>",
        mesh.json
    );
    h.push_str(JS);
    h
}

const CSS: &str = r#"<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Saira+Semi+Condensed:wght@500;700&family=Atkinson+Hyperlegible:wght@400;700&family=JetBrains+Mono:wght@400;600&display=swap">
<style>
/* Layout: summary tiles, then viewer + network side by side, charts, archive gallery, evidence. */
:root{--bg:#f3f4f6;--panel:#ffffff;--fg:#14181f;--fg2:#4a5260;--muted:#7a8291;--line:#dde1e7;--accent:#2a78d6;--ok:#0ca30c;--warn:#c98500;
--display:'Saira Semi Condensed','Arial Narrow',system-ui,sans-serif;--body:'Atkinson Hyperlegible',system-ui,-apple-system,'Segoe UI',sans-serif;--mono:'JetBrains Mono',ui-monospace,Menlo,Consolas,monospace}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){--bg:#0f1216;--panel:#171b21;--fg:#eef1f5;--fg2:#b9c0cb;--muted:#8a93a1;--line:#2a313b;--accent:#3987e5;--ok:#2fc12f;--warn:#e0a020;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#0f1216;--panel:#171b21;--fg:#eef1f5;--fg2:#b9c0cb;--muted:#8a93a1;--line:#2a313b;--accent:#3987e5;--ok:#2fc12f;--warn:#e0a020;color-scheme:dark}
body{background:var(--bg);color:var(--fg);font-family:var(--body);font-size:15px;line-height:1.5}
.wrap{max-width:1320px;margin:0 auto;padding-left:max(16px,env(safe-area-inset-left,0px));padding-right:max(16px,env(safe-area-inset-right,0px))}
.top{background:var(--panel);border-bottom:1px solid var(--line)}.top .wrap{padding-block:22px 18px}
.eyebrow{font-family:var(--mono);font-size:12px;letter-spacing:.08em;text-transform:uppercase;color:var(--muted)}
h1{font-family:var(--display);font-size:clamp(28px,4vw,40px);line-height:1.1;margin:.15em 0;text-wrap:balance}
h2{font-family:var(--display);font-size:20px;margin:0 0 10px;font-weight:700}
.sub{color:var(--fg2);margin:0;font-variant-numeric:tabular-nums}
.pill{display:inline-block;padding:1px 10px;border-radius:999px;font-weight:700;font-size:13px;border:1.5px solid currentColor}.pill.ok{color:var(--ok)}.pill.warn{color:var(--warn)}
main{display:grid;gap:18px;padding-block:18px 40px}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:12px}
.tile{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:12px 14px;min-width:0}
.lab{font-size:12px;color:var(--fg2);letter-spacing:.02em}.val{font-family:var(--display);font-size:30px;font-weight:700;font-variant-numeric:tabular-nums;line-height:1.15}.u{font-size:14px;color:var(--muted);margin-left:4px;font-family:var(--body)}.note{font-size:12px;color:var(--muted)}
.grid2{display:grid;grid-template-columns:minmax(0,1.25fr) minmax(0,1fr);gap:18px}
@media (max-width:900px){.grid2{grid-template-columns:minmax(0,1fr)}}
.panel{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:16px;min-width:0}
.ph{display:flex;justify-content:space-between;align-items:center;gap:10px;flex-wrap:wrap}
.scroll{overflow-x:auto}.scroll svg{max-width:100%;height:auto}
canvas#gl{width:100%;aspect-ratio:16/9;max-width:100%;display:block;border-radius:8px;background:#0b0e13;touch-action:none;cursor:grab}
.hint{font-size:12px;color:var(--muted);margin-top:6px}
.seg{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden}.seg button{font:inherit;font-size:13px;padding:5px 12px;background:transparent;color:var(--fg2);border:0;cursor:pointer}.seg button.on{background:var(--accent);color:#fff}.seg button:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.lede{color:var(--fg2);max-width:70ch;margin:0 0 12px}
.zoo{display:grid;grid-template-columns:repeat(auto-fill,minmax(min(100%,420px),1fr));gap:12px}
.card{margin:0;border:1px solid var(--line);border-radius:10px;overflow:hidden;background:var(--bg)}.card img{display:block;width:100%}.card figcaption{padding:8px 10px;font-size:13px;color:var(--fg2);font-variant-numeric:tabular-nums}
.card details{margin-top:6px}.card summary{cursor:pointer;color:var(--accent)}.card summary:focus-visible{outline:2px solid var(--accent)}
.ev dl{display:grid;grid-template-columns:max-content minmax(0,1fr);gap:6px 16px;margin:0}.ev dt{font-weight:700}.ev dd{margin:0;color:var(--fg2);min-width:0;overflow-wrap:anywhere}
code{font-family:var(--mono);font-size:12.5px}
@media (max-width:560px){.ev dl{grid-template-columns:minmax(0,1fr)}}
@media (prefers-reduced-motion: reduce){*{scroll-behavior:auto}}
</style>
"#;

const JS: &str = r#"<script>
(()=>{
const data=JSON.parse(document.getElementById('mesh').textContent);
const cv=document.getElementById('gl');const gl=cv.getContext('webgl2',{antialias:true});
if(!gl){cv.replaceWith(Object.assign(document.createElement('p'),{textContent:'WebGL2 is not available in this browser; the static figures below show the same design.'}));return;}
const PAL=[[42,120,214],[235,104,52],[27,175,122],[237,161,0],[232,123,164],[0,131,0],[98,80,214],[227,73,72]];
const b64=s=>{const b=atob(s),u=new Uint8Array(b.length);for(let i=0;i<b.length;i++)u[i]=b.charCodeAt(i);return u.buffer;};
const lo=data.lo,hi=data.hi;
function build(m,tint){
  const q=new Uint16Array(b64(m.pos)),own=new Uint8Array(b64(m.own)),idx=new Uint32Array(b64(m.idx));
  const n=m.n,P=new Float32Array(n*3),N=new Float32Array(n*3),C=new Float32Array(n*3);
  for(let i=0;i<n;i++)for(let k=0;k<3;k++)P[3*i+k]=lo[k]+(hi[k]-lo[k])*q[3*i+k]/65535;
  for(let t=0;t<idx.length;t+=3){const a=3*idx[t],b=3*idx[t+1],c=3*idx[t+2];
    const ux=P[b]-P[a],uy=P[b+1]-P[a+1],uz=P[b+2]-P[a+2],vx=P[c]-P[a],vy=P[c+1]-P[a+1],vz=P[c+2]-P[a+2];
    const nx=uy*vz-uz*vy,ny=uz*vx-ux*vz,nz=ux*vy-uy*vx;for(const v of[a,b,c]){N[v]+=nx;N[v+1]+=ny;N[v+2]+=nz;}}
  for(let i=0;i<n;i++){const o=own[i];const c=tint?tint:(o===255?[154,163,173]:PAL[o%PAL.length]);for(let k=0;k<3;k++)C[3*i+k]=Math.pow(c[k]/255,2.2);}
  const vao=gl.createVertexArray();gl.bindVertexArray(vao);
  for(const [loc,arr] of [[0,P],[1,N],[2,C]]){const b=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,b);gl.bufferData(gl.ARRAY_BUFFER,arr,gl.STATIC_DRAW);gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,3,gl.FLOAT,false,0,0);}
  const ib=gl.createBuffer();gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ib);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,idx,gl.STATIC_DRAW);
  return {vao,count:idx.length};
}
const vs=`#version 300 es
layout(location=0) in vec3 p;layout(location=1) in vec3 n;layout(location=2) in vec3 c;
uniform mat4 M;out vec3 wp;out vec3 wn;out vec3 col;void main(){wp=p;wn=normalize(n);col=c;gl_Position=M*vec4(p,1.0);}`;
const fs=`#version 300 es
precision highp float;in vec3 wp;in vec3 wn;in vec3 col;uniform vec3 eye;uniform float alpha;uniform float clipz;uniform vec3 cap;out vec4 o;
void main(){if(wp.z>clipz)discard;vec3 v=normalize(eye-wp);vec3 n=normalize(wn);bool back=dot(n,v)<0.0;if(back)n=-n;
vec3 L=normalize(vec3(-0.45,0.75,0.55));vec3 base=back&&clipz<1e8?cap:col;float lam=max(dot(n,L),0.0);float hemi=0.5+0.5*n.y;
vec3 h=normalize(L+v);float s=pow(max(dot(n,h),0.0),48.0)*0.5;float rim=pow(1.0-abs(dot(n,v)),3.0)*0.35;
vec3 c=base*(0.18+0.22*hemi+0.8*lam)+s+rim*(0.35+0.65*base);c=c/(1.0+0.6*c)*1.6;o=vec4(pow(clamp(c,0.0,1.0),vec3(1.0/2.2)),alpha);}`;
function sh(t,s){const x=gl.createShader(t);gl.shaderSource(x,s);gl.compileShader(x);return x;}
const pr=gl.createProgram();gl.attachShader(pr,sh(gl.VERTEX_SHADER,vs));gl.attachShader(pr,sh(gl.FRAGMENT_SHADER,fs));gl.linkProgram(pr);
const U=k=>gl.getUniformLocation(pr,k);
const fluid=build(data.fluid),part=build(data.part,[150,155,165]);
const ctr=[0,0,0];for(let k=0;k<3;k++)ctr[k]=(lo[k]+hi[k])/2;const span=Math.max(hi[0]-lo[0],hi[1]-lo[1],hi[2]-lo[2]);
let az=-0.5,el=0.35,dist=span*0.75,mode='cut';
function mat(){const a=cv.width/cv.height,f=1/Math.tan(0.3),nr=span*0.01,fr=span*6;
  const e=[ctr[0]+dist*Math.sin(az)*Math.cos(el),ctr[1]+dist*Math.sin(el),ctr[2]+dist*Math.cos(az)*Math.cos(el)];
  const z=[e[0]-ctr[0],e[1]-ctr[1],e[2]-ctr[2]];const zl=Math.hypot(...z);z.forEach((_,i)=>z[i]/=zl);
  let x=[z[2],0,-z[0]];const xl=Math.hypot(...x)||1;x=x.map(v=>v/xl);const y=[z[1]*x[2]-z[2]*x[1],z[2]*x[0]-z[0]*x[2],z[0]*x[1]-z[1]*x[0]];
  const V=[x[0],y[0],z[0],0,x[1],y[1],z[1],0,x[2],y[2],z[2],0,-(x[0]*e[0]+x[1]*e[1]+x[2]*e[2]),-(y[0]*e[0]+y[1]*e[1]+y[2]*e[2]),-(z[0]*e[0]+z[1]*e[1]+z[2]*e[2]),1];
  const P=[f/a,0,0,0,0,f,0,0,0,0,(fr+nr)/(nr-fr),-1,0,0,2*fr*nr/(nr-fr),0];const M=new Array(16).fill(0);
  for(let i=0;i<4;i++)for(let j=0;j<4;j++)for(let k=0;k<4;k++)M[j*4+i]+=P[k*4+i]*V[j*4+k];return {M,e};}
function draw(){const dpr=Math.min(devicePixelRatio||1,2);const w=cv.clientWidth*dpr|0,h=cv.clientHeight*dpr|0;if(cv.width!==w||cv.height!==h){cv.width=w;cv.height=h;}
  gl.viewport(0,0,cv.width,cv.height);gl.clearColor(0.043,0.055,0.075,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);gl.enable(gl.DEPTH_TEST);gl.useProgram(pr);
  const {M,e}=mat();gl.uniformMatrix4fv(U('M'),false,M);gl.uniform3fv(U('eye'),e);gl.uniform3fv(U('cap'),[0.8,0.77,0.7]);
  gl.disable(gl.BLEND);gl.depthMask(true);gl.uniform1f(U('alpha'),1);gl.uniform1f(U('clipz'),1e9);gl.bindVertexArray(fluid.vao);gl.drawElements(gl.TRIANGLES,fluid.count,gl.UNSIGNED_INT,0);
  if(mode==='cut'){gl.uniform1f(U('clipz'),0.0);gl.bindVertexArray(part.vao);gl.drawElements(gl.TRIANGLES,part.count,gl.UNSIGNED_INT,0);}
  else if(mode==='x'){gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.depthMask(false);gl.uniform1f(U('alpha'),0.18);gl.uniform1f(U('clipz'),1e9);gl.bindVertexArray(part.vao);gl.drawElements(gl.TRIANGLES,part.count,gl.UNSIGNED_INT,0);gl.depthMask(true);}}
let raf=0;const req=()=>{if(!raf)raf=requestAnimationFrame(()=>{raf=0;draw();});};
let drag=null;cv.addEventListener('pointerdown',ev=>{drag=[ev.clientX,ev.clientY];cv.setPointerCapture(ev.pointerId);});
cv.addEventListener('pointermove',ev=>{if(!drag)return;az-=(ev.clientX-drag[0])*0.008;el=Math.max(-1.4,Math.min(1.4,el+(ev.clientY-drag[1])*0.008));drag=[ev.clientX,ev.clientY];req();});
cv.addEventListener('pointerup',()=>drag=null);
cv.addEventListener('wheel',ev=>{ev.preventDefault();dist*=Math.exp(ev.deltaY*0.001);dist=Math.max(span*0.05,Math.min(span*3,dist));req();},{passive:false});
for(const [id,m] of [['m-cut','cut'],['m-x','x'],['m-f','f']])document.getElementById(id).addEventListener('click',ev=>{mode=m;document.querySelectorAll('.seg button').forEach(b=>b.classList.toggle('on',b===ev.currentTarget));req();});
addEventListener('resize',req);draw();
})();
</script>
"#;
