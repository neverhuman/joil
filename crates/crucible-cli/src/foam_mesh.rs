//! ASCII OpenFOAM mesh readers shared by the two mesh CLI ports.
use crate::port_support::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn tokens(path: &Path) -> Result<Vec<String>> {
    let raw = fs::read_to_string(path)?;
    let mut s = String::new();
    let mut rest = raw.as_str();
    while !rest.is_empty() {
        if rest.starts_with("/*") {
            let end = rest.find("*/").ok_or("unterminated mesh comment")?;
            rest = &rest[end + 2..];
        } else if rest.starts_with("//") {
            rest = match rest.find('\n') {
                Some(n) => &rest[n..],
                None => "",
            };
        } else {
            let c = rest.chars().next().ok_or("empty mesh")?;
            if "(){};".contains(c) {
                s.push(' ');
                s.push(c);
                s.push(' ');
            } else {
                s.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    let mut out: Vec<String> = s.split_whitespace().map(str::to_owned).collect();
    if let Some(start) = out.iter().position(|s| s == "FoamFile") {
        let end = out[start..]
            .iter()
            .position(|s| s == "}")
            .ok_or("missing mesh header end")?
            + start;
        out.drain(start..=end);
    }
    Ok(out)
}
pub fn points(folder: &Path) -> Result<Vec<[f64; 3]>> {
    let t = tokens(&folder.join("points"))?;
    let start = t
        .iter()
        .position(|s| s == "(")
        .ok_or("missing points list")?;
    let end = t
        .iter()
        .rposition(|s| s == ")")
        .ok_or("missing points end")?;
    let values = t[start + 1..end]
        .iter()
        .filter(|s| s.as_str() != "(" && s.as_str() != ")")
        .map(|s| s.parse::<f64>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    require(values.len() % 3 == 0, "invalid point coordinates")?;
    Ok(values.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect())
}
pub fn faces(folder: &Path) -> Result<Vec<Vec<usize>>> {
    let t = tokens(&folder.join("faces"))?;
    let mut result = Vec::new();
    let mut i = t
        .iter()
        .position(|s| s == "(")
        .ok_or("missing faces list")?
        + 1;
    while i < t.len() && t[i] != ")" {
        let _: usize = t[i].parse()?;
        require(t.get(i + 1).is_some_and(|s| s == "("), "invalid face")?;
        i += 2;
        let mut face = Vec::new();
        while i < t.len() && t[i] != ")" {
            face.push(t[i].parse()?);
            i += 1;
        }
        require(i < t.len(), "unterminated face")?;
        result.push(face);
        i += 1;
    }
    Ok(result)
}
pub fn key(points: &[[f64; 3]], face: &[usize]) -> Result<Vec<[u64; 3]>> {
    let result = face
        .iter()
        .map(|&i| points.get(i).copied().ok_or("face vertex out of bounds"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut result: Vec<_> = result.iter().map(point_key).collect();
    result.sort_by(|a, b| {
        for i in 0..3 {
            let order = a[i].cmp(&b[i]);
            if !order.is_eq() {
                return order;
            }
        }
        std::cmp::Ordering::Equal
    });
    Ok(result)
}
pub struct Options {
    pub root: PathBuf,
    pub case: PathBuf,
    pub source: PathBuf,
    pub length: f64,
    pub layers: i64,
}
pub fn options(extension: bool) -> Result<Options> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut case, mut source) = (None, None);
    let (mut length, mut layers) = (0.12, 60);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--root" => root = value.into(),
            "--case" => case = Some(PathBuf::from(value)),
            "--source" => source = Some(PathBuf::from(value)),
            "--length" if extension => length = value.parse()?,
            "--layers" if extension => layers = value.parse()?,
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    let root = root.canonicalize()?;
    let case = resolve(&case.ok_or("--case required")?)?;
    let source = resolve(&source.ok_or("--source required")?)?;
    require(
        case.starts_with(root.join("runs")) && source.starts_with(root.join("runs")),
        "project runs only",
    )?;
    Ok(Options {
        root,
        case,
        source,
        length,
        layers,
    })
}

// Python coordinate equality treats signed zeros as equal. Use exact float bits
// otherwise, so hashing never introduces a tolerance or changes connectivity.
pub fn point_key(p: &[f64; 3]) -> [u64; 3] {
    p.map(|v| if v == 0. { 0 } else { v.to_bits() })
}

pub fn json_bytes(value: &crate::ordered_json::Ordered) -> String {
    use crate::ordered_json::Ordered;
    fn encode(value: &Ordered, depth: usize) -> String {
        let (open, close, rows) = match value {
            Ordered::Scalar(serde_json::Value::Number(n)) if n.is_f64() => {
                let v = n.as_f64().expect("JSON floating point number");
                let raw = if v != 0. && (v.abs() < 1e-4 || v.abs() >= 1e16) {
                    format!("{v:e}")
                } else {
                    n.to_string()
                };
                return match raw.split_once('e') {
                    Some((mantissa, exponent)) => match exponent.parse::<i32>() {
                        Ok(e) => format!("{mantissa}e{e:+03}"),
                        Err(_) => raw,
                    },
                    None => raw,
                };
            }
            Ordered::Scalar(v) => return v.to_string(),
            Ordered::Array(a) => (
                "[",
                "]",
                a.iter().map(|v| encode(v, depth + 1)).collect::<Vec<_>>(),
            ),
            Ordered::Object(o) => (
                "{",
                "}",
                o.iter()
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            serde_json::Value::from(k.clone()),
                            encode(v, depth + 1)
                        )
                    })
                    .collect(),
            ),
        };
        if rows.is_empty() {
            format!("{open}{close}")
        } else {
            let indent = "  ".repeat(depth + 1);
            format!(
                "{open}\n{indent}{}\n{}{close}",
                rows.join(&format!(",\n{indent}")),
                "  ".repeat(depth)
            )
        }
    }
    let mut result = String::new();
    for c in (encode(value, 0) + "\n").chars() {
        if c.is_ascii() && c != '\u{7f}' {
            result.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                result.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    result
}
