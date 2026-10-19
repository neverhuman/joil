//! Deterministic geometry proposals; no performance selection.
#[allow(dead_code)]
#[path = "../foam_mesh.rs"]
mod foam_mesh;
#[allow(dead_code)]
#[path = "../ordered_json.rs"]
mod ordered_json;
#[allow(dead_code)]
#[path = "../port_support.rs"]
mod port_support;
use ordered_json::Ordered;
use port_support::*;
use std::f64::consts::PI;
use std::{fs, io::Write, path::PathBuf, process::ExitCode};
struct Random {
    state: [u32; 624],
    at: usize,
}
impl Random {
    fn new(seed: &str) -> Result<Self> {
        let digits = seed.trim_start_matches(['-', '+']);
        require(
            !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit()),
            "integer seed required",
        )?;
        let mut words = vec![0u32];
        for c in digits.bytes() {
            let mut carry = (c - b'0') as u64;
            for w in &mut words {
                let v = (*w as u64) * 10 + carry;
                *w = v as u32;
                carry = v >> 32;
            }
            if carry != 0 {
                words.push(carry as u32);
            }
        }
        let mut r = Self {
            state: [0; 624],
            at: 624,
        };
        r.state[0] = 19650218;
        for i in 1..624 {
            r.state[i] = 1812433253u32
                .wrapping_mul(r.state[i - 1] ^ (r.state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..624.max(words.len()) {
            r.state[i] = (r.state[i]
                ^ ((r.state[i - 1] ^ (r.state[i - 1] >> 30)).wrapping_mul(1664525)))
            .wrapping_add(words[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                r.state[0] = r.state[623];
                i = 1;
            }
            if j >= words.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            r.state[i] = (r.state[i]
                ^ ((r.state[i - 1] ^ (r.state[i - 1] >> 30)).wrapping_mul(1566083941)))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                r.state[0] = r.state[623];
                i = 1;
            }
        }
        r.state[0] = 0x80000000;
        Ok(r)
    }
    fn word(&mut self) -> u32 {
        if self.at >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x80000000) | (self.state[(i + 1) % 624] & 0x7fffffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 != 0 { 0x9908b0df } else { 0 };
            }
            self.at = 0;
        }
        let mut y = self.state[self.at];
        self.at += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }
    fn random(&mut self) -> f64 {
        let a = self.word() >> 5;
        let b = self.word() >> 6;
        ((a as f64) * 67108864. + b as f64) / 9007199254740992.
    }
    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }
    fn order(&mut self) -> u32 {
        loop {
            let v = self.word() >> 29;
            if v < 7 {
                return v + 2;
            }
        }
    }
}
fn run() -> Result<()> {
    let mut out = None;
    let mut seed = "20261015".to_string();
    let mut count = 8i64;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let v = args.next().ok_or("missing value")?;
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(v)),
            "--count" => count = v.parse()?,
            "--seed" => seed = v,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    let out = out.ok_or("--out required")?;
    let mut rng = Random::new(&seed)?;
    let baseline = sha(fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../baselines/ct35/genome.json"),
    )?);
    let program = |modes| {
        obj(vec![
            ("schema_version", val(1)),
            ("family", val("ct35-chamber-wall-field-v1")),
            ("baseline_file_sha256", val(baseline.clone())),
            ("modes", Ordered::Array(modes)),
        ])
    };
    let mut programmes = vec![program(vec![])];
    for i in 0..count {
        let mut modes = vec![];
        let mut amplitude_sum = 0.;
        for _ in 0..1 + i % 3 {
            let phase = rng.uniform(-PI, PI);
            let amplitude = rng.uniform(0.9, 1.7) / (1 + i % 3) as f64;
            let order = rng.order();
            let c = amplitude * phase.cos();
            let s = amplitude * phase.sin();
            let twist = rng.uniform(-2. * PI, 2. * PI);
            require(
                (2..=8).contains(&order)
                    && [c, s, twist].iter().all(|v| v.is_finite())
                    && twist.abs() <= 2. * PI,
                "invalid wall mode",
            )?;
            amplitude_sum += c.hypot(s);
            modes.push(obj(vec![
                ("azimuthal_order", val(order)),
                ("cos_mm", val(c)),
                ("sin_mm", val(s)),
                ("twist_rad", val(twist)),
            ]));
        }
        require(
            modes.len() <= 8 && amplitude_sum <= 1.8 + 1e-12,
            "wall-clearance proposal budget exceeded",
        )?;
        programmes.push(program(modes));
    }
    // The arbitrary-size seed is emitted as a canonical decimal JSON integer below.
    let result = obj(vec![
        ("schema_version", val(1)),
        ("seed", val("SEED_INTEGER")),
        ("operator", val("smooth-wall-mode-insertion-v1")),
        ("status", val("proposal_archive_not_performance_evolution")),
        ("fitness", val(serde_json::Value::Null)),
        (
            "selection",
            val("none; ordered deterministic proposals including zero-mutation baseline"),
        ),
        ("programmes", Ordered::Array(programmes)),
    ]);
    let digits = seed.trim_start_matches(['-', '+']).trim_start_matches('0');
    let canonical = if digits.is_empty() {
        "0".into()
    } else {
        format!("{}{}", if seed.starts_with('-') { "-" } else { "" }, digits)
    };
    let bytes = foam_mesh::json_bytes(&result).replacen("\"SEED_INTEGER\"", &canonical, 1);
    fs::create_dir_all(out.parent().ok_or("missing parent")?)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?
        .write_all(bytes.as_bytes())?;
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
