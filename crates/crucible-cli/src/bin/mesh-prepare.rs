//! Closed SI surface preparation for an explicitly exploratory full-liner mesh.
#[allow(dead_code)]
#[path = "carrier-configure.rs"]
mod configure;
use configure::{ordered_json, port_support};
use port_support::*;
use serde_json::{json, Map, Value};
use std::f64::consts::PI;
use std::{collections::BTreeMap, fs, path::PathBuf, process::ExitCode};
type Point = [f64; 3];
type Triangle = [Point; 3];
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn stl(data: &[u8]) -> Result<Vec<Triangle>> {
    let mut triangles = vec![];
    let binary = data
        .get(80..84)
        .map(|v| u32::from_le_bytes(v.try_into().expect("four bytes")) as usize)
        .filter(|n| n.checked_mul(50).and_then(|v| v.checked_add(84)) == Some(data.len()));
    if let Some(n) = binary {
        for i in 0..n {
            let mut t = [[0.; 3]; 3];
            for (j, p) in t.iter_mut().enumerate() {
                for (k, v) in p.iter_mut().enumerate() {
                    let offset = 84 + i * 50 + 12 + j * 12 + k * 4;
                    *v = f32::from_le_bytes(data[offset..offset + 4].try_into()?) as f64;
                }
            }
            triangles.push(t);
        }
    } else {
        let text = std::str::from_utf8(data)?;
        let mut vertices = vec![];
        for line in text.lines() {
            let mut words = line.split_whitespace();
            if words.next() == Some("vertex") {
                let v = words
                    .map(str::parse::<f64>)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                require(v.len() == 3, "invalid STL vertex")?;
                vertices.push([v[0], v[1], v[2]]);
            }
        }
        require(vertices.len() % 3 == 0, "invalid STL triangles")?;
        triangles = vertices
            .chunks_exact(3)
            .map(|v| [v[0], v[1], v[2]])
            .collect();
    }
    require(
        !triangles.is_empty() && triangles.iter().flatten().flatten().all(|v| v.is_finite()),
        "surface must be closed and outward",
    )?;
    Ok(triangles)
}
fn derive(p: &Map<String, Value>) -> Result<BTreeMap<&'static str, f64>> {
    let get = |k: &str| -> Result<f64> {
        p.get(k)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("missing numeric parameter {k}").into())
    };
    for v in p.values() {
        require(
            v.as_f64().is_some_and(f64::is_finite),
            "nonfinite parameter",
        )?;
    }
    let angle = get("cone_half_angle_deg")?;
    require(
        get("n_inlets")? == 2. && angle > 0. && angle < 89.,
        "invalid inlet count or cone angle",
    )?;
    for (k, v) in p {
        let v = v.as_f64().ok_or("nonfinite parameter")?;
        require(
            if k == "inlet_z_offset_mm" {
                v >= 0.
            } else {
                v > 0.
            },
            "invalid dimension",
        )?;
    }
    let d2 = get("d2_mm")?;
    let d1 = d2 * get("d1_over_d2")?;
    let d3 = d2 * get("d3_over_d2")?;
    require(d1 > d2 && d2 > d3 && d3 > 0., "invalid diameters")?;
    let (r1, r2, r3) = (d1 / 2., d2 / 2., d3 / 2.);
    let a = angle.to_radians();
    let l1 = d1 * get("l1_over_d1")?;
    let lc = (r1 - r2) / a.tan();
    let l2 = d2 * get("l2_over_d2")?;
    let l3 = d3 * get("l3_over_d3")?;
    let area = get("inlet_area_ratio")? * PI * r1.powi(2) / 2.;
    let w = (area / get("inlet_h_over_w")?).sqrt();
    let h = w * get("inlet_h_over_w")?;
    let ro = d2 * get("do_over_d2")? / 2.;
    require(
        !(w >= r1
            || get("inlet_z_offset_mm")? + h >= l1
            || get("inlet_length_cfd_mm")?.min(get("inlet_length_part_mm")?) <= r1
            || get("head_spigot_over_d2")? * d2 >= l2
            || get("plenum_radius_mm")? <= ro),
        "inlet, collar or plenum does not fit",
    )?;
    let frustum = |l: f64, ra: f64, rb: f64| PI * l * (ra.powi(2) + ra * rb + rb.powi(2)) / 3.;
    let primitive =
        |y: f64| (y * (r1 * r1 - y * y).max(0.).sqrt() + r1 * r1 * (y / r1).asin()) / 2.;
    let volume = PI * r1 * r1 * l1
        + frustum(lc, r1, r2)
        + frustum(l2, r2, r3)
        + PI * r3 * r3 * l3
        + 2. * h * (get("inlet_length_cfd_mm")? * w - primitive(r1) + primitive(r1 - w))
        + PI * ro * ro * 2. * ro * get("orifice_length_over_do")?
        + PI * get("plenum_radius_mm")?.powi(2) * get("plenum_length_mm")?;
    Ok(BTreeMap::from([
        ("r1", r1),
        ("r2", r2),
        ("r3", r3),
        ("ro", ro),
        ("l1", l1),
        ("lc", lc),
        ("l2", l2),
        ("l3", l3),
        ("a", a),
        ("zc", l1 + lc),
        ("zt", l1 + lc + l2),
        ("ze", l1 + lc + l2 + l3),
        ("zh", l1 + lc + get("head_spigot_over_d2")? * d2),
        ("w", w),
        ("h", h),
        ("area", area),
        ("analytic_fluid_volume_mm3", volume),
    ]))
}
fn run() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let (mut case, mut surface, mut genome) = (None, None, None);
    let mut level = 0u32;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let v = args.next().ok_or("missing option value")?;
        match a.as_str() {
            "--case" => case = Some(PathBuf::from(v)),
            "--surface" => surface = Some(PathBuf::from(v)),
            "--genome" => genome = Some(PathBuf::from(v)),
            "--level" => level = v.parse()?,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    require(level <= 1, "level must be 0 or 1")?;
    let case = resolve(&case.ok_or("--case required")?)?;
    require(
        case.starts_with(root.join("runs")) && !case.exists(),
        "create a fresh case within runs; never overwrite evidence",
    )?;
    let surface = surface.ok_or("--surface required")?;
    let genome = genome.unwrap_or(root.join("baselines/ct35/genome.json"));
    let genome_bytes = fs::read(genome)?;
    let obj: Value = serde_json::from_slice(&genome_bytes)?;
    require(
        obj["schema_version"] == 1 && obj["family"] == "ct35-parametric-v1",
        "unsupported genome",
    )?;
    let p = obj["parameters"].as_object().ok_or("missing parameters")?;
    let d = derive(p)?;
    let param = |k: &str| p[k].as_f64().expect("validated parameters");
    let data = fs::read(surface)?;
    let triangles = stl(&data)?;
    // trimesh merges STL vertices at its default eight-decimal merge precision.
    let key = |p: Point| p.map(|v| (v * 1e8).round_ties_even() as i64);
    let mut edges: BTreeMap<([i64; 3], [i64; 3]), (usize, i32)> = BTreeMap::new();
    let mut volume = 0.;
    let mut normals = vec![];
    let mut areas = vec![];
    for t in &triangles {
        for i in 0..3 {
            let a = key(t[i]);
            let b = key(t[(i + 1) % 3]);
            let (e, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let entry = edges.entry(e).or_default();
            entry.0 += 1;
            entry.1 += sign;
        }
        let c = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        let norm = dot(c, c).sqrt();
        areas.push(norm / 2.);
        normals.push(if norm > 0. {
            c.map(|v| v / norm)
        } else {
            [0.; 3]
        });
        volume += dot(t[0], cross(t[1], t[2])) / 6.;
    }
    require(
        edges.values().all(|&(n, w)| n == 2 && w == 0) && volume > 0.,
        "surface must be closed and outward",
    )?;
    let max_z = triangles
        .iter()
        .flatten()
        .map(|v| v[2])
        .fold(f64::NEG_INFINITY, f64::max);
    require(
        (max_z - d["ze"] * 0.001).abs() <= 1e-8,
        "surface scale or full-liner length differs from genome",
    )?;
    let planes = [
        (
            "inlet1",
            0,
            -param("inlet_length_cfd_mm") * 0.001,
            d["area"] * 1e-6,
            [-1., 0., 0.],
        ),
        (
            "inlet2",
            0,
            param("inlet_length_cfd_mm") * 0.001,
            d["area"] * 1e-6,
            [1., 0., 0.],
        ),
        (
            "overflow",
            2,
            -(2. * d["ro"] * param("orifice_length_over_do") + param("plenum_length_mm")) * 0.001,
            PI * param("plenum_radius_mm").powi(2) * 1e-6,
            [0., 0., -1.],
        ),
        (
            "underflow",
            2,
            d["ze"] * 0.001,
            PI * d["r3"].powi(2) * 1e-6,
            [0., 0., 1.],
        ),
    ];
    let mut labels = vec!["wall"; triangles.len()];
    let mut patches = Map::new();
    for (name, axis, plane, expected, normal) in planes {
        let f = plane.abs() as f32;
        let ulp = (f32::from_bits(f.to_bits() + 1) - f) as f64;
        let tol = (2. * ulp).max(1e-9);
        let mut area = 0.;
        let mut count = 0;
        let mut alignment = 1f64;
        for (i, t) in triangles.iter().enumerate() {
            if t.iter().all(|v| (v[axis] - plane).abs() < tol) {
                labels[i] = name;
                area += areas[i];
                count += 1;
                alignment = alignment.min(dot(normals[i], normal));
            }
        }
        require(
            count > 0 && (area / expected - 1.).abs() <= 0.002 && alignment >= 0.999,
            &format!("terminal {name} has wrong area or orientation"),
        )?;
        patches.insert(name.into(),json!({"area_m2":area,"relative_area_error":area/expected-1.,"triangles":count,"normal":normal.map(|v|v as i32),"plane_axis":axis,"plane_position_m":plane,"plane_tolerance_m":tol}));
    }
    let mut labelled = String::new();
    for name in ["wall", "inlet1", "inlet2", "overflow", "underflow"] {
        labelled.push_str(&format!("solid {name}\n"));
        for (i, t) in triangles.iter().enumerate() {
            if labels[i] != name {
                continue;
            }
            let vector = |v: Point| v.map(|x| configure::g(x, 12)).join(" ");
            labelled.push_str(&format!(
                " facet normal {}\n  outer loop\n",
                vector(normals[i])
            ));
            for v in t {
                labelled.push_str(&format!("   vertex {}\n", vector(*v)));
            }
            labelled.push_str("  endloop\n endfacet\n");
        }
        labelled.push_str(&format!("endsolid {name}\n"));
    }
    fs::create_dir_all(case.parent().ok_or("missing parent")?)?;
    fs::create_dir(&case)?;
    fs::create_dir_all(case.join("constant/triSurface"))?;
    fs::write(case.join("constant/triSurface/ct35.stl"), &labelled)?;
    fs::create_dir(case.join("system"))?;
    {
        fs::write(
            case.join("system/controlDict"),
            concat!(
                "FoamFile { format ascii; class dictionary; object controlDict; }\n",
                r#"application snappyHexMesh;
startFrom startTime; startTime 0; stopAt endTime; endTime 1000; deltaT 1;
writeControl timeStep; writeInterval 1000; purgeWrite 2;
writeFormat ascii; writePrecision 12; writeCompression off; runTimeModifiable false;"#,
                "\n"
            ),
        )?;
    }
    {
        fs::write(
            case.join("system/blockMeshDict"),
            concat!(
                "FoamFile { format ascii; class dictionary; object blockMeshDict; }\n",
                r#"convertToMeters 1;
vertices ((-.072 -.048 -.036) (.072 -.048 -.036) (.072 .048 -.036) (-.072 .048 -.036)
          (-.072 -.048 1.656) (.072 -.048 1.656) (.072 .048 1.656) (-.072 .048 1.656));
blocks (hex (0 1 2 3 4 5 6 7) (24 16 282) simpleGrading (1 1 1));
edges (); boundary (background {type patch; faces ((0 3 2 1)(4 5 6 7)(0 1 5 4)(1 2 6 5)(2 3 7 6)(3 0 4 7));});
mergePatchPairs ();"#,
                "\n"
            ),
        )?;
    }
    if level == 0 {
        fs::write(
            case.join("system/snappyHexMeshDict"),
            concat!(
                "FoamFile { format ascii; class dictionary; object snappyHexMeshDict; }\n",
                r#"castellatedMesh true; snap true; addLayers false;
geometry {
 ct35 { type triSurfaceMesh; file "ct35.stl"; name ct35; regions { wall { name wall; }
inlet1 { name inlet1; }
inlet2 { name inlet2; }
overflow { name overflow; }
underflow { name underflow; } } }
 head {type searchableBox; min (-.064 -.039 -.026); max (.064 .039 .19);}
 bore {type searchableBox; min (-.002 -.002 -.004); max (.002 .002 .003);}
}
castellatedMeshControls {
 maxLocalCells 800000; maxGlobalCells 1200000; minRefinementCells 0; nCellsBetweenLevels 3;
 features ();
 refinementSurfaces {ct35 {level (1 1); regions { wall { level (2 2); patchInfo { type wall; } }
inlet1 { level (3 3); patchInfo { type patch; } }
inlet2 { level (3 3); patchInfo { type patch; } }
overflow { level (3 3); patchInfo { type patch; } }
underflow { level (3 3); patchInfo { type patch; } } } } }
 resolveFeatureAngle 30;
 refinementRegions {head {mode inside; level 2;} bore {mode inside; level 5;}}
 insidePoint (0.00013 0.00017 .03513); allowFreeStandingZoneFaces true;
}
snapControls {nSmoothPatch 3; tolerance 1.5; nSolveIter 40; nRelaxIter 5;
 nFeatureSnapIter 10; implicitFeatureSnap true; explicitFeatureSnap false; multiRegionFeatureSnap true;}
addLayersControls {relativeSizes true; layers {} expansionRatio 1.2; finalLayerThickness .3;
 minThickness .1; nGrow 0; featureAngle 60; nRelaxIter 5; nSmoothSurfaceNormals 1;
 nSmoothNormals 3; nSmoothThickness 10; maxFaceThicknessRatio .5; maxThicknessToMedialRatio .3;
 minMedianAxisAngle 90; nBufferCellsNoExtrude 0; nLayerIter 50;}
meshQualityControls {maxNonOrtho 65; maxBoundarySkewness 20; maxInternalSkewness 4;
 maxConcave 80; minFlatness .5; minVol 1e-18; minTetQuality 1e-15; minArea -1; minTwist .02;
 minDeterminant .001; minFaceWeight .02; minVolRatio .01; minTriangleTwist -1;
 nSmoothScale 4; errorReduction .75; relaxed {maxNonOrtho 75;}}
writeFlags (scalarLevels layerSets layerFields); mergeTolerance 1e-7;"#,
                "\n"
            ),
        )?;
    }
    if level == 1 {
        fs::write(
            case.join("system/snappyHexMeshDict"),
            concat!(
                "FoamFile { format ascii; class dictionary; object snappyHexMeshDict; }\n",
                r#"castellatedMesh true; snap true; addLayers false;
geometry {
 ct35 { type triSurfaceMesh; file "ct35.stl"; name ct35; regions { wall { name wall; }
inlet1 { name inlet1; }
inlet2 { name inlet2; }
overflow { name overflow; }
underflow { name underflow; } } }
 head {type searchableBox; min (-.064 -.039 -.026); max (.064 .039 .19);}
 bore {type searchableBox; min (-.002 -.002 -.004); max (.002 .002 .003);}
}
castellatedMeshControls {
 maxLocalCells 800000; maxGlobalCells 1200000; minRefinementCells 0; nCellsBetweenLevels 3;
 features ();
 refinementSurfaces {ct35 {level (2 2); regions { wall { level (3 3); patchInfo { type wall; } }
inlet1 { level (4 4); patchInfo { type patch; } }
inlet2 { level (4 4); patchInfo { type patch; } }
overflow { level (4 4); patchInfo { type patch; } }
underflow { level (4 4); patchInfo { type patch; } } } } }
 resolveFeatureAngle 30;
 refinementRegions {head {mode inside; level 3;} bore {mode inside; level 6;}}
 insidePoint (0.00013 0.00017 .03513); allowFreeStandingZoneFaces true;
}
snapControls {nSmoothPatch 3; tolerance 1.5; nSolveIter 40; nRelaxIter 5;
 nFeatureSnapIter 10; implicitFeatureSnap true; explicitFeatureSnap false; multiRegionFeatureSnap true;}
addLayersControls {relativeSizes true; layers {} expansionRatio 1.2; finalLayerThickness .3;
 minThickness .1; nGrow 0; featureAngle 60; nRelaxIter 5; nSmoothSurfaceNormals 1;
 nSmoothNormals 3; nSmoothThickness 10; maxFaceThicknessRatio .5; maxThicknessToMedialRatio .3;
 minMedianAxisAngle 90; nBufferCellsNoExtrude 0; nLayerIter 50;}
meshQualityControls {maxNonOrtho 65; maxBoundarySkewness 20; maxInternalSkewness 4;
 maxConcave 80; minFlatness .5; minVol 1e-18; minTetQuality 1e-15; minArea -1; minTwist .02;
 minDeterminant .001; minFaceWeight .02; minVolRatio .01; minTriangleTwist -1;
 nSmoothScale 4; errorReduction .75; relaxed {maxNonOrtho 75;}}
writeFlags (scalarLevels layerSets layerFields); mergeTolerance 1e-7;"#,
                "\n"
            ),
        )?;
    }
    {
        fs::write(
            case.join("system/fvSchemes"),
            concat!(
                "FoamFile { format ascii; class dictionary; object fvSchemes; }\n",
                r#"ddtSchemes {default steadyState;} gradSchemes {default Gauss linear;} divSchemes {default none;} laplacianSchemes {default Gauss linear corrected;} interpolationSchemes {default linear;} snGradSchemes {default corrected;}"#,
                "\n"
            ),
        )?;
    }
    {
        fs::write(
            case.join("system/fvSolution"),
            concat!(
                "FoamFile { format ascii; class dictionary; object fvSolution; }\n",
                r#"solvers {}"#,
                "\n"
            ),
        )?;
    }
    let manifest = json!({"schema_version":1,"kind":"full_liner_exploratory_mesh_input","units":"m","genome_file_sha256":sha(&genome_bytes),"surface_sha256":sha(&data),"labelled_surface_sha256":sha(labelled.as_bytes()),"surface_volume_m3":volume,"analytic_volume_m3":d["analytic_fluid_volume_mm3"]*1e-9,"patches":patches,"refinement_increment":level,"limitations":["mesh geometry and numerical convergence must be measured after meshing","no boundary layers in first exploratory mesh","not a qualified CFD evaluator"]});
    let bytes = serde_json::to_string_pretty(&manifest)?;
    fs::write(case.join("mesh-input.json"), format!("{bytes}\n"))?;
    println!("{bytes}");
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::FAILURE
        }
    }
}
