//! Conservative software-only status importer; preserves legacy output identity.
use joil_evidence::identity::{scientific_hash, strict_json};
#[allow(dead_code)] // Each CLI uses a different subset of the ordered codec.
#[path = "../ordered_json.rs"]
mod ordered_json;
use ordered_json::Ordered;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
type Result<T> = std::result::Result<T, String>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn fields(v: &Value, keys: &[&str]) -> Result<()> {
    for key in keys {
        require(v.get(key).is_some(), &format!("missing field: {key}"))?;
    }
    Ok(())
}
fn rows(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or("expected array".into())
}
fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or("expected string".into())
}
fn bytes(p: &Path) -> Result<Vec<u8>> {
    fs::read(p).map_err(|e| e.to_string())
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn pinned(root: &Path, r: &Value) -> Result<Value> {
    let p = root
        .join(text(&r["path"])?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    require(p.starts_with(root), "registry path escapes repository")?;
    let raw = bytes(&p)?;
    require(
        r["sha256"] == sha(&raw),
        &format!("stale or changed evidence: {}", text(&r["path"])?),
    )?;
    strict_json(&raw)
}
fn contract(root: &Path) -> Result<(Value, Value, String)> {
    let raw = fs::read_to_string(root.join("requirements/joil-hydrocyclone-v1.toml"))
        .map_err(|e| e.to_string())?;
    let t: toml::Value = toml::from_str(&raw).map_err(|e| e.to_string())?;
    // Check TOML floats before JSON conversion can turn nonfinite values into null.
    let thresholds = t
        .get("thresholds")
        .and_then(toml::Value::as_table)
        .ok_or("missing thresholds")?;
    for (k, v) in thresholds {
        let vs = if let Some(a) = v.as_array() {
            a.clone()
        } else {
            vec![v.clone()]
        };
        require(
            vs.iter()
                .all(|v| v.as_integer().is_some() || v.as_float().is_some_and(f64::is_finite)),
            &format!("invalid finite requirement threshold: {k}"),
        )?;
    }
    let req = serde_json::to_value(t).map_err(|e| e.to_string())?;
    fields(
        &req,
        &[
            "admission",
            "blockers",
            "thresholds",
            "preregistration",
            "baseline",
            "objectives",
            "scenarios",
            "hypervolume",
            "quantile_policy",
        ],
    )?;
    require(
        [
            "sandbox_only",
            "admitted",
            "suspended",
            "superseded",
            "closed",
        ]
        .iter()
        .any(|s| req["admission"] == *s),
        "unknown campaign admission state",
    )?;
    require(
        req["admission"] != "admitted"
            || (rows(&req["blockers"])?.is_empty()
                && [
                    "pressure_budget_pa",
                    "energy_budget_j_per_treated_m3",
                    "absolute_pressure_min_pa",
                    "duty_band_m3_h",
                ]
                .iter()
                .all(|k| req["thresholds"].get(k).is_some())),
        "cannot admit unresolved physical requirements",
    )?;
    let raw = bytes(&root.join("experiments/wave2-preregistration-v1.json"))?;
    require(
        req["preregistration"]["sha256"] == sha(&raw),
        "preregistration changed without requirement revision",
    )?;
    require(
        req["baseline"]["sha256"] == sha(&bytes(&root.join("baselines/ct35/genome.json"))?),
        "baseline changed without requirement revision",
    )?;
    let prereg = strict_json(&raw)?;
    let mut expected = vec![];
    for s in rows(&prereg["objectives_minimize_5d"])? {
        let grade = s["id"] == "one_minus_grade_20um_p05";
        let unit = match text(&s["units"])? {
            "mg/L" => "mg_per_litre_dispersed_oil",
            "J/m^3" => "joule_per_treated_cubic_metre",
            "-" => "dimensionless",
            "um" => "micrometre",
            "Pa" => "pascal",
            _ => return Err("unknown unit".into()),
        };
        expected.push(json!({"id":s["id"],"source_id":if grade {json!("grade_20um_p05")} else {s["id"].clone()},"sense":s["sense"],"unit":unit,"transform":if grade {"one_minus"} else {"identity"},"reference_point":null}));
    }
    let mut actual = rows(&req["objectives"])?.clone();
    for s in &mut actual {
        s.as_object_mut()
            .ok_or("expected objective")?
            .entry("reference_point")
            .or_insert(Value::Null);
    }
    require(
        actual == expected,
        "preregistered/executed objective IDs/order/units/sense/transform/reference drift",
    )?;
    let scenarios = rows(&req["scenarios"])?;
    require(
        scenarios.iter().map(|s| &s["id"]).collect::<Vec<_>>()
            == rows(&prereg["held_out_operating_duties"])?
                .iter()
                .map(|s| &s["duty_id"])
                .collect::<Vec<_>>(),
        "scenario inventory drift",
    )?;
    require(
        scenarios
            .iter()
            .all(|s| s["sha256"] == req["preregistration"]["sha256"]),
        "scenario distribution identity drift",
    )?;
    let h = &req["hypervolume"];
    require(
        h["scope"] == "sandbox_only_new_policy_not_legacy_preregistered"
            && h["objective_ids"] == json!(actual.iter().map(|s| &s["id"]).collect::<Vec<_>>())
            && rows(&h["reference_point"])?.len() == actual.len()
            && rows(&h["normalization"])?.len() == actual.len()
            && rows(&h["reference_point"])?
                .iter()
                .all(|n| n.as_f64().is_some_and(f64::is_finite))
            && rows(&h["normalization"])?
                .iter()
                .all(|n| n.as_f64().is_some_and(|n| n.is_finite() && n > 0.0)),
        "invalid frozen sandbox hypervolume policy",
    )?;
    require(
        req["quantile_policy"]
            == json!({"interpretation":"empirical_distribution_free_two_sided_order_statistic_ci","confidence":0.95,"min_n_p05_p95":72,"min_n_p01_p99":368}),
        "unsupported or weakened tail quantile policy",
    )?;
    let actual = json!(actual);
    let hash = scientific_hash(&actual)?;
    Ok((req, actual, hash))
}
fn generate(root: &Path) -> Result<Ordered> {
    let raw = bytes(&root.join("crates/joil-evidence/registry/status-registry.json"))?;
    let registry = strict_json(&raw)?;
    fields(
        &registry,
        &[
            "schema",
            "software",
            "observations",
            "scientific_claims",
            "historical_status",
            "requirements",
        ],
    )?;
    require(registry["schema"] == 1, "unsupported status registry")?;
    let (req, objectives, hash) = contract(root)?;
    let mut software = vec![];
    for entry in rows(&registry["software"])? {
        fields(entry, &["id", "receipt"])?;
        let receipt = pinned(root, &entry["receipt"])?;
        let empty = json!([]);
        let checks = rows(receipt.get("checks").unwrap_or(&empty))?;
        software.push(json!({"id":entry["id"],"state":if !checks.is_empty() && checks.iter().all(|c|c["exit_code"]==0 || c["exit_code"]==false || c["exit_code"].as_f64()==Some(0.0)){"software_contract_verified"}else{"incomplete"},"receipt":entry["receipt"]}));
    }
    require(
        rows(&registry["observations"])?.is_empty(),
        "scientific observations require a trusted Rust resolver receipt; importer not admitted",
    )?;
    let mut claims = Map::new();
    for e in rows(&registry["scientific_claims"])? {
        fields(e, &["id", "required_observations"])?;
        claims.insert(
            text(&e["id"])?.into(),
            json!({"state":"incomplete","missing":e["required_observations"]}),
        );
    }
    let historical = pinned(root, &registry["historical_status"])?;
    if root.join("runs").exists() {
        let mut paths = fs::read_dir(root.join("runs"))
            .map_err(|e| e.to_string())?
            .map(|e| e.map(|e| e.path().join("campaign.json")))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        paths.sort();
        for p in paths {
            if p.is_file() {
                let receipt = strict_json(&bytes(&p)?)?;
                require(
                    receipt["schema"] == "joil.campaign.v1" && receipt["objective_sha256"] == hash,
                    "campaign objective hash mismatch",
                )?;
                require(
                    receipt["executed_objectives"] == objectives,
                    "campaign executable objective contract mismatch",
                )?;
            }
        }
    }
    // Explicit insertion order is the Python public byte contract.
    let mut out = vec![];
    for (k, v) in [
        ("schema_version", json!(2)),
        ("generated_by", json!("tools/gen_status.py")),
        ("evidence_registry_sha256", json!(sha(&raw))),
        (
            "phase",
            json!("software_truth_repair_scientific_claim_inventory_incomplete"),
        ),
        ("campaign_admission", req["admission"].clone()),
        ("campaign_blockers", req["blockers"].clone()),
        (
            "requirements_sha256",
            json!(sha(&bytes(&root.join(text(&registry["requirements"])?))?)),
        ),
        ("objective_sha256", json!(hash)),
        ("software", json!(software)),
        ("claims", Value::Object(claims)),
        ("qualified_baseline", json!(false)),
        ("structural_qualification", json!(false)),
        ("physical_validation", json!(false)),
        ("oil_grade_evaluated", json!(false)),
        ("qualified_fitness", Value::Null),
        ("qualified_capacity_gain", Value::Null),
        ("customer_or_site_data_public", json!(false)),
        ("legacy_status_is_historical_not_authoritative", json!(true)),
        ("legacy_status", historical),
    ] {
        let value = match k {
            "software" => Ordered::Array(
                rows(&v)?
                    .iter()
                    .map(|e| {
                        Ordered::Object(
                            ["id", "state", "receipt"]
                                .iter()
                                .map(|k| {
                                    (
                                        (*k).into(),
                                        if *k == "receipt" {
                                            Ordered::Object(
                                                ["path", "sha256"]
                                                    .iter()
                                                    .map(|k| {
                                                        (
                                                            (*k).into(),
                                                            Ordered::from_value(
                                                                e["receipt"][k].clone(),
                                                            ),
                                                        )
                                                    })
                                                    .collect(),
                                            )
                                        } else {
                                            Ordered::from_value(e[k].clone())
                                        },
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            "claims" => Ordered::Object(
                rows(&registry["scientific_claims"])?
                    .iter()
                    .map(|e| {
                        let id = e["id"].as_str().unwrap();
                        (
                            id.into(),
                            Ordered::Object(
                                ["state", "missing"]
                                    .iter()
                                    .map(|k| ((*k).into(), Ordered::from_value(v[id][k].clone())))
                                    .collect(),
                            ),
                        )
                    })
                    .collect(),
            ),
            "legacy_status" => serde_json::from_slice(&bytes(
                &root.join(text(&registry["historical_status"]["path"])?),
            )?)
            .map_err(|e| e.to_string())?,
            _ => Ordered::from_value(v),
        };
        out.push((k.into(), value));
    }
    Ok(Ordered::Object(out))
}
// Python ensure_ascii=True preserves the frozen public bytes, including surrogate pairs.
fn encoded(v: &Ordered) -> Result<String> {
    let s = v.pretty(0);
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() {
            out.push(c)
        } else {
            let mut units = [0; 2];
            for u in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{u:04x}"));
            }
        }
    }
    out.push('\n');
    Ok(out)
}
fn run(root: &Path, check: bool) -> Result<()> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let expected = encoded(&generate(&root)?)?;
    let path = root.join("docs/STATUS.json");
    if check {
        require(
            fs::read_to_string(path).map_err(|e| e.to_string())? == expected,
            "docs/STATUS.json is stale or hand-edited; run python3 tools/gen_status.py",
        )?
    } else {
        fs::write(path, expected).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    let mut root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--root" => root = args.next().unwrap_or_default().into(),
            _ => {
                eprintln!("usage: gen-status [--check] [--root ROOT]");
                std::process::exit(2)
            }
        }
    }
    match run(&root, check) {
        Ok(()) => println!("PASS: generated status and preregistered objective identities agree"),
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1)
        }
    }
}
