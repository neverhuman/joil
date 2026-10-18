//! Source-bound software control for D-28 coordination ownership policy.
use sha2::{Digest, Sha256};
use std::process::{Command, Stdio};
const BASELINE: &str = "e1e9203872f07e0f7cd47f50c7a3da6b4fb5ffc0";
const SOURCES: [(&str, &str); 2] = [
    (
        "docs/COORDINATION.md",
        "08b88ff8ea3853745fac524fa387f6dc90ed65b441e49b24f228b2d40ca72b0a",
    ),
    (
        "COMMS.md",
        "f64dd4d8a716ac65b2c10c21f79147eda749560dc8d9ec73fbf7674d24a6af0b",
    ),
];
fn git(args: &[&str], capture: bool) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let argv = std::iter::once("git")
            .chain(args.iter().copied())
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Command '[{argv}]' returned non-zero exit status {}.",
            out.status.code().unwrap_or(1)
        ));
    }
    Ok(out.stdout)
}
fn allocation_passes(acknowledged: bool, leased: bool, paths: &[&str], others: &[&str]) -> bool {
    acknowledged
        && leased
        && !paths.is_empty()
        && paths.iter().all(|a| {
            others.iter().all(|b| {
                a != b
                    && !a.starts_with(&format!("{}/", b.trim_end_matches('/')))
                    && !b.starts_with(&format!("{}/", a.trim_end_matches('/')))
            })
        })
}
fn run(mode: &str) -> Result<i32, String> {
    git(
        &["cat-file", "-e", &format!("{BASELINE}^{{commit}}")],
        false,
    )?;
    println!("verified frozen source commit {BASELINE}");
    for (path, expected) in SOURCES {
        let bytes = git(&["show", &format!("{BASELINE}:{path}")], true)?;
        let actual = format!("{:x}", Sha256::digest(bytes));
        if actual != expected {
            return Err(format!("frozen source digest mismatch: {path}: {actual}"));
        }
        println!("verified {path} sha256={actual}");
    }
    let accepted = allocation_passes(
        mode == "positive",
        true,
        &["example/worker-owned.py"],
        &["example/integrator-owned.py"],
    );
    if mode == "positive" {
        if !accepted {
            return Err("acknowledged, leased, disjoint allocation rejected".into());
        }
        println!("PASS: synthetic allocation with target acknowledgement, active lease, and disjoint write scope accepted");
        Ok(0)
    } else {
        if accepted {
            return Err("unacknowledged proposed allocation was incorrectly accepted".into());
        }
        println!("PASS: synthetic allocation proposal without target acknowledgement rejected");
        Ok(1)
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [flag, mode] if flag == "--mode" && ["positive", "red"].contains(&mode.as_str()) => mode,
        _ => {
            eprintln!("usage: v5-d28-ownership-control --mode {{positive,red}}");
            std::process::exit(2);
        }
    };
    std::process::exit(match run(mode) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("FAIL: {error}");
            2
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lease_acknowledgement_and_path_ownership_are_required() {
        assert!(allocation_passes(true, true, &["a/b"], &["a/c"]));
        for (ack, lease, paths, others) in [
            (false, true, vec!["a/b"], vec!["c"]),
            (true, false, vec!["a/b"], vec!["c"]),
            (true, true, vec![], vec!["c"]),
            (true, true, vec!["a/b"], vec!["a/b"]),
            (true, true, vec!["a/b"], vec!["a/"]),
            (true, true, vec!["a/"], vec!["a/b"]),
        ] {
            assert!(!allocation_passes(ack, lease, &paths, &others));
        }
    }
}
