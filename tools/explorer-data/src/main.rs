//! Thin wrapper — delegates all logic to `soma_schema::explorer::build_json`.
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(
        std::env::args().nth(1).unwrap_or_else(|| "examples/showcase".to_string()),
    );
    let out = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "website/data/migrations.json".to_string());

    let json = soma_schema::explorer::build_json(&root).expect("failed to build explorer JSON");

    if out == "-" {
        print!("{json}");
    } else {
        std::fs::write(&out, &json).expect("failed to write output");
        eprintln!("Wrote {out}");
    }
}
