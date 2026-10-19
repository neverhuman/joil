//! glTF 2.0 binary (`GLB`) and `3MF` archive writers, retention closure verification, and export attempt accounting.

use super::*;

pub(super) fn glb(surface: &TaggedSurface, points: &[Point]) -> Result<(Vec<u8>, f64)> {
    let mut binary = Vec::new();
    let mut error: f64 = 0.0;
    let mut quantized = Vec::new();
    for p in points {
        let q = p.map(|x| x as f32);
        if q.iter().any(|x| !x.is_finite()) {
            return Err(unsupported("glTF float32 coordinate overflow"));
        }
        let qm = q.map(f64::from);
        error = error.max(norm(sub(qm, *p)));
        quantized.push(qm);
        for x in q {
            binary.extend_from_slice(&x.to_le_bytes());
        }
    }
    // A display conversion must not silently collapse a valid governing facet.
    for f in &surface.boundary().facets {
        let [a, b, c] = f.vertices.map(|i| quantized[i]);
        let [x, y, z] = f.vertices.map(|i| points[i]);
        if dot(cross(sub(b, a), sub(c, a)), cross(sub(y, x), sub(z, x))) <= 0.0 {
            return Err(unsupported("glTF float32 collapses/reverses a facet"));
        }
    }
    let bbox = bounding_box(&quantized);
    let positions_len = binary.len();
    let mut views =
        vec![json!({"buffer":0,"byteOffset":0,"byteLength":positions_len,"target":34962})];
    let mut accessors = vec![
        json!({"bufferView":0,"componentType":5126,"count":points.len(),"type":"VEC3","min":bbox[0],"max":bbox[1]}),
    ];
    let patch_names: BTreeSet<_> = surface
        .boundary()
        .facets
        .iter()
        .map(|f| f.tag.patch.clone())
        .collect();
    let mut primitives = Vec::new();
    for name in patch_names {
        let facets: Vec<_> = surface
            .boundary()
            .facets
            .iter()
            .filter(|f| f.tag.patch == name)
            .collect();
        let offset = binary.len();
        for f in &facets {
            for i in f.vertices {
                binary.extend_from_slice(&(i as u32).to_le_bytes());
            }
        }
        let index = views.len();
        views.push(
            json!({"buffer":0,"byteOffset":offset,"byteLength":binary.len()-offset,"target":34963}),
        );
        accessors.push(
            json!({"bufferView":index,"componentType":5125,"count":facets.len()*3,"type":"SCALAR"}),
        );
        primitives.push(json!({"attributes":{"POSITION":0},"indices":index,"mode":4,"extras":{"tag":facets[0].tag}}));
    }
    let doc = json!({"asset":{"version":"2.0","generator":"crucible-geom bounded-facet-export/1"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
        "meshes":[{"primitives":primitives}],"buffers":[{"byteLength":binary.len()}],"bufferViews":views,"accessors":accessors,
        "extras":{"units":"metre","boundary_sha256":surface.boundary_sha256(),"coordinate_axes":"preserved from partition; orientation recorded by transform","float32_error_bound_m":error}});
    let mut json_bytes = serde_json::to_vec(&doc).map_err(|e| invalid(&e.to_string()))?;
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + json_bytes.len() + 8 + binary.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4e4f534au32.to_le_bytes());
    out.extend(json_bytes);
    out.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004e4942u32.to_le_bytes());
    out.extend(binary);
    Ok((out, error))
}
pub(super) fn three_mf(
    surface: &TaggedSurface,
    points: &[Point],
    unit: BuildUnit,
) -> Result<Vec<u8>> {
    let mut model = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><model unit="{}" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"><metadata name="Title">partition {}</metadata><resources><object id="1" type="model"><mesh><vertices>"#,
        unit.name(),
        surface.boundary_sha256()
    );
    for p in points {
        let p = p.map(|x| x * unit.per_metre());
        if p.iter().any(|x| !x.is_finite()) {
            return Err(invalid("3MF build unit overflow"));
        }
        model.push_str(&format!(
            r#"<vertex x="{}" y="{}" z="{}"/>"#,
            p[0], p[1], p[2]
        ));
    }
    model.push_str("</vertices><triangles>");
    for f in &surface.boundary().facets {
        let [a, b, c] = f.vertices;
        model.push_str(&format!(r#"<triangle v1="{a}" v2="{b}" v3="{c}"/>"#));
    }
    model.push_str(
        "</triangles></mesh></object></resources><build><item objectid=\"1\"/></build></model>",
    );
    let content=br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
    let relationships=br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#;
    Ok(zip_stored(&[
        ("[Content_Types].xml", content),
        ("_rels/.rels", relationships),
        ("3D/3dmodel.model", model.as_bytes()),
    ]))
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !c
}
/// Minimal ZIP store writer, single disk, no ZIP64, fixed 1980 date. The bounded
/// surface route keeps every member far below u32 lengths. CRCs cover raw bytes.
fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, bytes) in entries {
        let offset = out.len() as u32;
        let crc = crc32(bytes);
        let len = bytes.len() as u32;
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        for x in [20u16, 0, 0, 0, 33] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        for x in [crc, len, len] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(bytes);
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        for x in [20u16, 20, 0, 0, 0, 33] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        for x in [crc, len, len] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        for x in [name.len() as u16, 0, 0, 0, 0] {
            central.extend_from_slice(&x.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let offset = out.len() as u32;
    let len = central.len() as u32;
    out.extend(central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    for x in [0u16, 0, entries.len() as u16, entries.len() as u16] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    for x in [len, offset] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetainedRole {
    RawInput,
    Provenance,
    GoverningCfdMesh,
    GoverningFeaMesh,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetainedObject {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub role: RetainedRole,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetentionClosure {
    pub objects: Vec<RetainedObject>,
}
impl RetentionClosure {
    /// Verify actual archive bytes before *any* external cleanup decision. A
    /// scalar receipt, symlink, path traversal or missing governing mesh fails.
    /// This method performs no deletion and grants no physical qualification.
    pub fn verify(
        &self,
        archive: &Path,
        required: &BTreeSet<RetainedRole>,
        expected_governing: &BTreeMap<RetainedRole, String>,
    ) -> Result<()> {
        if required.is_empty()
            || !required.contains(&RetainedRole::RawInput)
            || !required.contains(&RetainedRole::Provenance)
            || !required.iter().any(|r| {
                matches!(
                    r,
                    RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
                )
            })
        {
            return Err(invalid(
                "retention requires raw input, provenance and governing mesh",
            ));
        }
        // Expected governing identities come from the accepted subject manifest,
        // independently of this archive receipt. Relabeling a scalar as a mesh
        // cannot satisfy those identities.
        for role in required.iter().filter(|r| {
            matches!(
                r,
                RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
            )
        }) {
            if !expected_governing.get(role).is_some_and(|h| hash_valid(h)) {
                return Err(invalid("accepted governing mesh identity is mandatory"));
            }
        }
        let mut paths = BTreeSet::new();
        let mut roles = BTreeSet::new();
        if fs::symlink_metadata(archive)?.file_type().is_symlink() {
            return Err(invalid("archive symlink"));
        }
        for object in &self.objects {
            let relative = Path::new(&object.path);
            if !hash_valid(&object.sha256)
                || object.bytes == 0
                || relative.components().count() != 1
                || !matches!(relative.components().next(), Some(Component::Normal(_)))
                || !paths.insert(&object.path)
            {
                return Err(invalid("unsafe/duplicate retention object"));
            }
            let path = archive.join(relative);
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != object.bytes
            {
                return Err(invalid("missing or replaced retention object"));
            }
            if sha256(&fs::read(path)?) != object.sha256 {
                return Err(invalid("retention digest mismatch"));
            }
            if matches!(
                object.role,
                RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
            ) && expected_governing.get(&object.role) != Some(&object.sha256)
            {
                return Err(invalid(
                    "archive does not contain the accepted governing mesh",
                ));
            }
            roles.insert(object.role.clone());
        }
        if !required.is_subset(&roles) {
            return Err(invalid("scalar-only or incomplete retention closure"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportAttemptOutcome {
    AcceptedArtifact,
    NumericalArtifactFailure(String),
    Unsupported(String),
    InvalidInput(String),
    InfrastructureFailure(String),
}
pub type AttemptOutcome = ExportAttemptOutcome;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub fixture: String,
    pub input_sha256: String,
    pub compile_ns: u128,
    pub mesh_ns: u128,
    pub elapsed_ns: u128,
    pub outcome: AttemptOutcome,
    pub retention: RetentionClosure,
    pub physical_infeasibility: bool,
}
/// Actual stage costs, failure diagnosis and raw inputs survive every attempt.
/// Returned mesh bytes are the governing object, never a scalar stand-in.
/// Corpus definitions and source identities are supplied in retained raw input.
pub fn measure_attempt<C>(
    fixture: &str,
    raw_input: &[u8],
    destination: &Path,
    role: RetainedRole,
    compile: impl FnOnce(&[u8]) -> Result<C>,
    mesh: impl FnOnce(C) -> Result<Vec<u8>>,
) -> Result<AttemptRecord> {
    if fixture.is_empty()
        || raw_input.is_empty()
        || !matches!(
            role,
            RetainedRole::GoverningCfdMesh | RetainedRole::GoverningFeaMesh
        )
    {
        return Err(invalid("benchmark fixture/raw input/governing role"));
    }
    fresh_run(destination)?;
    write_new(&destination.join("raw-input.json"), raw_input)?;
    let start = Instant::now();
    let compile_start = Instant::now();
    let compiled = compile(raw_input);
    let compile_ns = compile_start.elapsed().as_nanos();
    let (output, mesh_ns) = match compiled {
        Ok(compiled) => {
            let mesh_start = Instant::now();
            let output = mesh(compiled);
            (output, mesh_start.elapsed().as_nanos())
        }
        Err(error) => (Err(error), 0),
    };
    let mut objects = vec![RetainedObject {
        path: "raw-input.json".into(),
        sha256: sha256(raw_input),
        bytes: raw_input.len() as u64,
        role: RetainedRole::RawInput,
    }];
    let outcome = match output {
        Ok(bytes) if !bytes.is_empty() => {
            write_new(&destination.join("governing-mesh.json"), &bytes)?;
            objects.push(RetainedObject {
                path: "governing-mesh.json".into(),
                sha256: sha256(&bytes),
                bytes: bytes.len() as u64,
                role,
            });
            AttemptOutcome::AcceptedArtifact
        }
        Ok(_) => AttemptOutcome::NumericalArtifactFailure("empty governing mesh".into()),
        Err(ArtifactError::NumericalFailure(s)) => AttemptOutcome::NumericalArtifactFailure(s),
        Err(ArtifactError::Unsupported(s)) => AttemptOutcome::Unsupported(s),
        Err(ArtifactError::Invalid(s)) => AttemptOutcome::InvalidInput(s),
        Err(ArtifactError::Io(s)) => AttemptOutcome::InfrastructureFailure(s),
    };
    let provenance = bytes_json(
        &json!({"fixture":fixture,"input_sha256":sha256(raw_input),"compile_ns":compile_ns,"mesh_ns":mesh_ns,"outcome":outcome,"physical_infeasibility":false}),
    )?;
    write_new(&destination.join("provenance.json"), &provenance)?;
    objects.push(RetainedObject {
        path: "provenance.json".into(),
        sha256: sha256(&provenance),
        bytes: provenance.len() as u64,
        role: RetainedRole::Provenance,
    });
    let record = AttemptRecord {
        fixture: fixture.into(),
        input_sha256: sha256(raw_input),
        compile_ns,
        mesh_ns,
        elapsed_ns: start.elapsed().as_nanos(),
        outcome,
        retention: RetentionClosure { objects },
        physical_infeasibility: false,
    };
    write_new(&destination.join("attempt.json"), &bytes_json(&record)?)?;
    Ok(record)
}
