//! Internal insertion-order JSON for Python-compatible CLI output.
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{fmt, fs, path::Path};
type Result<T> = std::result::Result<T, String>;
// Keep input insertion order for Python-compatible embedded dashboard bytes.
#[derive(Clone)]
pub(crate) enum Ordered {
    Object(Vec<(String, Ordered)>),
    Array(Vec<Ordered>),
    Scalar(Value),
}
impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Ordered;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("JSON")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Ordered, A::Error> {
                let mut v = vec![];
                while let Some(p) = a.next_entry()? {
                    v.push(p)
                }
                Ok(Ordered::Object(v))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Ordered, A::Error> {
                let mut v = vec![];
                while let Some(p) = a.next_element()? {
                    v.push(p)
                }
                Ok(Ordered::Array(v))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Ordered, E> {
                Ok(Ordered::Scalar(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
impl Ordered {
    pub(crate) fn parse(path: &Path) -> Result<Self> {
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn field(&mut self, k: &str) -> Result<&mut Self> {
        if let Self::Object(m) = self {
            m.iter_mut()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v)
                .ok_or(format!("'{k}'"))
        } else {
            Err("expected object".into())
        }
    }
    pub(crate) fn array(&mut self) -> Result<&mut Vec<Self>> {
        if let Self::Array(a) = self {
            Ok(a)
        } else {
            Err("expected array".into())
        }
    }
    pub(crate) fn put(&mut self, k: String, v: Self) -> Result<()> {
        if let Self::Object(m) = self {
            if let Some((_, old)) = m.iter_mut().find(|(n, _)| n == &k) {
                *old = v
            } else {
                m.push((k, v))
            }
            Ok(())
        } else {
            Err("expected object".into())
        }
    }
    pub(crate) fn pretty(&self, depth: usize) -> String {
        let indent = "  ".repeat(depth + 1);
        let (open, close, rows) = match self {
            Self::Scalar(v) => return v.to_string(),
            Self::Array(a) => (
                "[",
                "]",
                a.iter().map(|v| v.pretty(depth + 1)).collect::<Vec<_>>(),
            ),
            Self::Object(m) => (
                "{",
                "}",
                m.iter()
                    .map(|(k, v)| format!("{}: {}", Value::from(k.clone()), v.pretty(depth + 1)))
                    .collect(),
            ),
        };
        if rows.is_empty() {
            format!("{open}{close}")
        } else {
            format!(
                "{open}\n{indent}{}\n{}{close}",
                rows.join(&format!(",\n{indent}")),
                "  ".repeat(depth)
            )
        }
    }
    pub(crate) fn encode(&self, sorted: bool) -> String {
        let sep = if sorted { ", " } else { "," };
        match self {
            Self::Scalar(v) => v.to_string(),
            Self::Array(a) => format!(
                "[{}]",
                a.iter()
                    .map(|v| v.encode(sorted))
                    .collect::<Vec<_>>()
                    .join(sep)
            ),
            Self::Object(m) => {
                let mut entries = m.iter().collect::<Vec<_>>();
                if sorted {
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                }
                format!(
                    "{{{}}}",
                    entries
                        .iter()
                        .map(|(k, v)| format!(
                            "{}{}{}",
                            Value::from(k.clone()),
                            if sorted { ": " } else { ":" },
                            v.encode(sorted)
                        ))
                        .collect::<Vec<_>>()
                        .join(sep)
                )
            }
        }
    }
}
impl Ordered {
    pub(crate) fn from_value(v: Value) -> Self {
        match v {
            Value::Array(a) => Self::Array(a.into_iter().map(Self::from_value).collect()),
            Value::Object(o) => Self::Object(
                o.into_iter()
                    .map(|(k, v)| (k, Self::from_value(v)))
                    .collect(),
            ),
            v => Self::Scalar(v),
        }
    }
}
