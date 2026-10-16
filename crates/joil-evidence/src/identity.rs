//! Content-bound scientific identity. Runtime metadata never participates in its hash.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;

pub const WIRE: &str = "joil.science.canonical.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRef {
    pub id: String,
    pub revision: String,
    pub sha256: String,
}

impl ContentRef {
    pub fn check(&self) -> Result<(), String> {
        check_id(&self.id)?;
        if self.revision.trim().is_empty() || !is_hash(&self.sha256) {
            return Err("invalid content identity".into());
        }
        Ok(())
    }
    pub fn from_bytes(id: &str, revision: &str, bytes: &[u8]) -> Self {
        Self {
            id: id.into(),
            revision: revision.into(),
            sha256: crate::hex(&Sha256::digest(bytes)),
        }
    }
}
pub fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn check_id(s: &str) -> Result<(), String> {
    if s.trim().is_empty() || s.len() > 512 || s.chars().any(char::is_control) {
        Err("invalid identity".into())
    } else {
        Ok(())
    }
}
pub fn unique_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        check_id(id)?;
        if !seen.insert(id.nfc().collect::<String>()) {
            return Err("duplicate_or_normalization_collision".into());
        }
    }
    Ok(())
}
pub fn same_id(a: &str, b: &str) -> bool {
    a.nfc().eq(b.nfc())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SubjectRef {
    Compiled {
        bundle: Box<crate::subject::SubjectBundle>,
    },
    Genome {
        representation: ContentRef,
        bits: Vec<String>,
    },
    NativeCad {
        artifact: ContentRef,
        geometry_state: ContentRef,
        format: CadFormat,
        unit: LengthUnit,
    },
    Specimen {
        id: String,
        passport: ContentRef,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CadFormat {
    Step,
    Stl,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    Millimetre,
    Metre,
}
impl SubjectRef {
    pub fn refs(&self) -> Vec<&ContentRef> {
        match self {
            Self::Compiled { bundle } => bundle.refs(),
            Self::Genome { representation, .. } => vec![representation],
            Self::NativeCad {
                artifact,
                geometry_state,
                ..
            } => vec![artifact, geometry_state],
            Self::Specimen { passport, .. } => vec![passport],
        }
    }
    pub fn check(&self) -> Result<(), String> {
        for r in self.refs() {
            r.check()?;
        }
        match self {
            Self::Compiled { bundle } => bundle.check()?,
            Self::Genome { bits, .. } => {
                if bits.is_empty() {
                    return Err("empty genome".into());
                }
                for b in bits {
                    if b.len() != 16
                        || !b
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                    {
                        return Err("invalid genome bits".into());
                    }
                    if !f64::from_bits(u64::from_str_radix(b, 16).map_err(|_| "invalid bits")?)
                        .is_finite()
                    {
                        return Err("nonfinite genome".into());
                    }
                }
            }
            Self::Specimen { id, .. } => check_id(id)?,
            _ => (),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterialState {
    Identified { passport: ContentRef },
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScientificIdentity {
    pub schema: u32,
    pub subject: SubjectRef,
    pub requirements: Vec<ContentRef>,
    pub source: ContentRef,
    pub build: ContentRef,
    pub cargo_lock: ContentRef,
    pub config: ContentRef,
    pub evaluator: ContentRef,
    pub scenario: ContentRef,
    pub objective: ContentRef,
    pub descriptor: ContentRef,
    pub solver_build: ContentRef,
    pub model: ContentRef,
    pub numerics: ContentRef,
    pub process: ContentRef,
    pub material: MaterialState,
    pub dependencies: Vec<ContentRef>,
}
impl ScientificIdentity {
    pub fn refs(&self) -> Vec<&ContentRef> {
        let mut out = self.subject.refs();
        out.extend(self.requirements.iter());
        out.extend([
            &self.source,
            &self.build,
            &self.cargo_lock,
            &self.config,
            &self.evaluator,
            &self.scenario,
            &self.objective,
            &self.descriptor,
        ]);
        out.extend([
            &self.solver_build,
            &self.model,
            &self.numerics,
            &self.process,
        ]);
        out.extend(self.dependencies.iter());
        if let MaterialState::Identified { passport } = &self.material {
            out.push(passport);
        }
        out
    }
    pub fn check(&self) -> Result<(), String> {
        if self.schema != 2 || self.requirements.is_empty() {
            return Err("unsupported schema or missing requirements".into());
        }
        self.subject.check()?;
        let mut seen = BTreeMap::new();
        let mut normalized = BTreeMap::new();
        for r in self.refs() {
            r.check()?;
            if let Some(previous) = normalized.insert(r.id.nfc().collect::<String>(), r.id.clone())
            {
                if previous != r.id {
                    return Err("normalization_collision".into());
                }
            }
            if let Some(previous) = seen.insert(&r.id, (&r.revision, &r.sha256)) {
                if previous != (&r.revision, &r.sha256) {
                    return Err("conflicting content identity".into());
                }
            }
        }
        unique_ids(self.requirements.iter().map(|r| r.id.as_str()))?;
        if let MaterialState::Unknown { reason } = &self.material {
            if reason.trim().is_empty() {
                return Err("unknown material needs reason".into());
            }
        }
        Ok(())
    }
    pub fn sha256(&self) -> Result<String, String> {
        self.check()?;
        scientific_hash(self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMetadata {
    pub host: String,
    pub threads: u32,
    pub metadata: BTreeMap<String, String>,
}

/// The frozen wire encodes numeric type and exact bits; it never rounds a decision scalar.
pub fn canonical_wire(v: &Value) -> Result<Vec<u8>, String> {
    fn add(v: &Value, b: &mut Vec<u8>) -> Result<(), String> {
        match v {
            Value::Null => b.push(b'n'),
            Value::Bool(x) => b.push(if *x { b't' } else { b'f' }),
            Value::String(s) => {
                b.extend(format!("s{}:", s.len()).bytes());
                b.extend(s.as_bytes());
            }
            Value::Number(n) => {
                if let Some(x) = n.as_u64() {
                    b.extend(format!("u{x};").bytes());
                } else if let Some(x) = n.as_i64() {
                    b.extend(format!("i{x};").bytes());
                } else {
                    let x = n.as_f64().ok_or("invalid number")?;
                    if !x.is_finite() {
                        return Err("nonfinite number".into());
                    }
                    b.extend(format!("d{:016x};", x.to_bits()).bytes());
                }
            }
            Value::Array(a) => {
                b.extend(format!("a{}:", a.len()).bytes());
                for x in a {
                    add(x, b)?;
                }
            }
            Value::Object(o) => {
                let mut keys = std::collections::BTreeSet::new();
                for key in o.keys() {
                    if !keys.insert(key.nfc().collect::<String>()) {
                        return Err("normalization_collision".into());
                    }
                }
                b.extend(format!("o{}:", o.len()).bytes());
                let sorted: BTreeMap<_, _> = o.iter().collect();
                for (k, v) in sorted {
                    add(&Value::String(k.clone()), b)?;
                    add(v, b)?;
                }
            }
        }
        Ok(())
    }
    let mut b = Vec::new();
    add(v, &mut b)?;
    Ok(b)
}
pub fn scientific_hash<T: Serialize>(v: &T) -> Result<String, String> {
    let json = serde_json::to_value(v).map_err(|e| e.to_string())?;
    Ok(crate::hex(&Sha256::digest(canonical_wire(&json)?)))
}

/// Bounded strict JSON with duplicate keys and exact Rust binary64 parsing.
pub fn strict_json(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > 1_048_576 {
        return Err("payload too large".into());
    }
    struct Parser<'a> {
        b: &'a [u8],
        i: usize,
        nodes: usize,
    }
    impl Parser<'_> {
        fn space(&mut self) {
            while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\r' | b'\n') {
                self.i += 1;
            }
        }
        fn string(&mut self) -> Result<String, String> {
            let begin = self.i;
            self.i += 1;
            while self.i < self.b.len() {
                match self.b[self.i] {
                    b'\\' => self.i += 2,
                    b'"' => {
                        self.i += 1;
                        return serde_json::from_slice(&self.b[begin..self.i])
                            .map_err(|e| e.to_string());
                    }
                    _ => self.i += 1,
                }
            }
            Err("unfinished string".into())
        }
        fn value(&mut self, depth: usize) -> Result<Value, String> {
            self.nodes += 1;
            if depth > 64 || self.nodes > 100_000 {
                return Err("payload bounds".into());
            }
            self.space();
            let start = self.i;
            let c = *self.b.get(self.i).ok_or("missing value")?;
            if c == b'"' {
                return self.string().map(Value::String);
            }
            if c == b'[' || c == b'{' {
                self.i += 1;
                self.space();
                let close = if c == b'[' { b']' } else { b'}' };
                let mut a = Vec::new();
                let mut o = serde_json::Map::new();
                let mut keys = std::collections::BTreeSet::new();
                if self.b.get(self.i) != Some(&close) {
                    loop {
                        self.space();
                        if c == b'{' {
                            if self.b.get(self.i) != Some(&b'"') {
                                return Err("object key required".into());
                            }
                            let key = self.string()?;
                            self.space();
                            if self.b.get(self.i) != Some(&b':') {
                                return Err("colon required".into());
                            }
                            self.i += 1;
                            if !keys.insert(key.nfc().collect::<String>()) {
                                return Err("duplicate_or_normalization_collision".into());
                            }
                            o.insert(key, self.value(depth + 1)?);
                        } else {
                            a.push(self.value(depth + 1)?);
                        }
                        self.space();
                        if self.b.get(self.i) == Some(&close) {
                            break;
                        }
                        if self.b.get(self.i) != Some(&b',') {
                            return Err("separator required".into());
                        }
                        self.i += 1;
                    }
                }
                self.i += 1;
                return Ok(if c == b'[' {
                    Value::Array(a)
                } else {
                    Value::Object(o)
                });
            }
            while self.i < self.b.len()
                && !matches!(
                    self.b[self.i],
                    b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t'
                )
            {
                self.i += 1;
            }
            let token = std::str::from_utf8(&self.b[start..self.i]).map_err(|e| e.to_string())?;
            if matches!(token, "null" | "true" | "false") {
                return serde_json::from_str(token).map_err(|e| e.to_string());
            }
            if !matches!(token.as_bytes().first(), Some(b'-' | b'0'..=b'9')) {
                return Err("invalid literal".into());
            }
            // Reuse serde's grammar-only number scanner, then std's correctly rounded binary64 parse.
            serde_json::from_str::<serde::de::IgnoredAny>(token).map_err(|e| e.to_string())?;
            if token.contains(['.', 'e', 'E']) || token == "-0" {
                let x = token.parse::<f64>().map_err(|e| e.to_string())?;
                let mantissa = token.split(['e', 'E']).next().unwrap();
                if x == 0.0 && mantissa.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
                    return Err("number underflow".into());
                }
                return serde_json::Number::from_f64(x)
                    .map(Value::Number)
                    .ok_or("nonfinite number".into());
            }
            if token.starts_with('-') {
                token
                    .parse::<i64>()
                    .map(|x| Value::Number(x.into()))
                    .map_err(|_| "integer out of range".into())
            } else {
                token
                    .parse::<u64>()
                    .map(|x| Value::Number(x.into()))
                    .map_err(|_| "integer out of range".into())
            }
        }
    }
    let mut p = Parser {
        b: bytes,
        i: 0,
        nodes: 0,
    };
    let v = p.value(0)?;
    p.space();
    if p.i != bytes.len() {
        return Err("trailing payload".into());
    }
    Ok(v)
}
pub fn strict_decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(strict_json(bytes)?).map_err(|e| e.to_string())
}
