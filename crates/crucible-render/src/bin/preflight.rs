use std::{env, fs, path::PathBuf, process};

fn main() {
    let mut args = env::args().skip(1);
    let bundle = args.next().map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: preflight BUNDLE [--glb FILE --subject FILE]");
        process::exit(2);
    });
    let mut glb = None;
    let mut subject = None;
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_else(|| {
            eprintln!("missing value for {flag}");
            process::exit(2);
        });
        match flag.as_str() {
            "--glb" => glb = Some(PathBuf::from(value)),
            "--subject" => subject = Some(PathBuf::from(value)),
            _ => {
                eprintln!("unknown argument: {flag}");
                process::exit(2);
            }
        }
    }
    let bytes = fs::read(&bundle).unwrap_or_else(|error| {
        eprintln!("bundle read failed: {error}");
        process::exit(1);
    });
    match crucible_render::validate_adapter_input(&bytes, glb.as_deref(), subject.as_deref()) {
        Ok(scene) => println!(
            "{{\"valid\":true,\"evidence_class\":\"{:?}\",\"observable\":{}}}",
            scene.evidence_class,
            serde_json::to_string(&scene.observable).expect("serialize string")
        ),
        Err(error) => {
            eprintln!("scene preflight rejected input: {error}");
            process::exit(1);
        }
    }
}
