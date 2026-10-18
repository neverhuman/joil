//! Publication guard. Paths come from Git, including nonignored untracked files.
use std::{fs, path::Path, process::Command};

fn violations(name: &str, data: &[u8], deny: &[Vec<u8>]) -> Vec<&'static str> {
    let mut errors = Vec::new();
    let path = Path::new(name);
    let base = path.file_name().unwrap_or_default().to_string_lossy();
    if path.components().any(|p| {
        [
            ".private",
            ".venv",
            "runs",
            "artifacts",
            "node_modules",
            "target",
            ".env",
        ]
        .iter()
        .any(|x| p.as_os_str() == *x)
    }) || (base.starts_with(".env") && base != ".env.example")
    {
        errors.push("private/generated path");
    }
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if ["zip", "stl", "step", "sif", "blend", "pem", "key"].contains(&ext.as_str()) {
        errors.push("raw/generated or credential file");
    }
    if data.len() > 2_000_000 {
        errors.push("large file needs explicit publication review");
    }
    let key = ["OPENSSH", "RSA", "EC", "DSA"].iter().any(|kind| {
        let marker = format!("-----BEGIN {kind} PRIVATE KEY-----");
        data.windows(marker.len()).any(|w| w == marker.as_bytes())
    });
    let token = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_"]
        .iter()
        .any(|prefix| {
            data.windows(prefix.len() + 30).any(|w| {
                w.starts_with(prefix.as_bytes())
                    && w[prefix.len()..].iter().all(u8::is_ascii_alphanumeric)
            })
        });
    if key || token {
        errors.push("credential pattern");
    }
    let lower = data.to_ascii_lowercase();
    if deny
        .iter()
        .filter(|d| !d.is_empty())
        .any(|d| lower.windows(d.len()).any(|w| w == d.to_ascii_lowercase()))
    {
        errors.push("private publication deny rule");
    }
    errors
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let rules = root.join(".private/publication-deny.txt");
    if root.join(".private/source").exists() && !rules.exists() {
        println!("FAIL: private source archive exists without publication deny rules");
        return Ok(false);
    }
    let deny: Vec<Vec<u8>> = if rules.exists() {
        fs::read_to_string(&rules)?
            .lines()
            .map(|s| s.trim().as_bytes().to_vec())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(&root)
        .output()?;
    if !output.status.success() {
        return Err("git ls-files failed".into());
    }
    let mut count = 0;
    let mut passed = true;
    for raw in output.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(raw)?;
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        count += 1;
        for reason in violations(name, &fs::read(path)?, &deny) {
            println!("{name}: {reason}");
            passed = false;
        }
    }
    if passed {
        println!(
            "PASS: {count} public candidate files checked; private deny rules applied={}",
            if deny.is_empty() { "False" } else { "True" }
        );
    }
    Ok(passed)
}

fn main() {
    match run() {
        Ok(true) => (),
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_boundaries() {
        assert!(violations("README.md", b"research", &[]).is_empty());
        assert!(violations(".env.example", b"example", &[]).is_empty());
        assert_eq!(
            violations(".private/x", b"", &[]),
            ["private/generated path"]
        );
        assert_eq!(
            violations("x.STEP", b"", &[]),
            ["raw/generated or credential file"]
        );
        assert_eq!(
            violations("x", b"Client", &[b"client".to_vec()]),
            ["private publication deny rule"]
        );
        assert_eq!(
            violations("x", &[b'x'; 2_000_001], &[]),
            ["large file needs explicit publication review"]
        );
        assert!(violations("x", &[b'x'; 2_000_000], &[]).is_empty());
        let token = format!("ghp_{}", "a".repeat(30));
        assert_eq!(
            violations("x", token.as_bytes(), &[]),
            ["credential pattern"]
        );
        assert!(violations("x", &token.as_bytes()[..33], &[]).is_empty());
    }
}
