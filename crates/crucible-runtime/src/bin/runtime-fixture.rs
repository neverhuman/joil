//! Synthetic owned child capability. No native physical evaluator is present.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--fixture-child" {
        std::process::exit(2);
    }
    match crucible_runtime::task::fixture_child(std::path::Path::new(&args[2])) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(70);
        }
    }
}
