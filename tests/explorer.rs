//! Explorer end-to-end test — no database required.
//!
//! Calls `soma_schema::build_json` against `examples/showcase` and asserts
//! the returned JSON contains expected content.
//!
//! Run with: `cargo test --features explorer --test explorer`
#![cfg(feature = "explorer")]

use soma_schema::build_json;

/// Locate the showcase example directory relative to the workspace root.
fn showcase_path() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR points to the crate root where Cargo.toml lives.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    std::path::PathBuf::from(manifest)
        .join("examples")
        .join("showcase")
}

#[test]
fn test_explorer_json_contains_expected_tables() {
    let path = showcase_path();
    let json = build_json(&path).expect("build_json should succeed on examples/showcase");

    // Core tables defined in the vault schema.
    assert!(
        json.contains("\"organizations\""),
        "JSON should mention the organizations table"
    );
    assert!(
        json.contains("\"users\""),
        "JSON should mention the users table"
    );
    assert!(
        json.contains("\"secrets\""),
        "JSON should mention the secrets table"
    );
}

#[test]
fn test_explorer_json_contains_fk_relation() {
    let path = showcase_path();
    let json = build_json(&path).expect("build_json should succeed on examples/showcase");

    // The schema has FKs (e.g. secrets.environment_id -> environments.id,
    // environments.project_id -> projects.id). The JSON encodes relations as objects
    // with from_table / to_table keys.
    assert!(
        json.contains("\"from_table\""),
        "JSON should contain FK relation entries with a from_table key"
    );
    assert!(
        json.contains("\"to_table\""),
        "JSON should contain FK relation entries with a to_table key"
    );
}

#[test]
fn test_explorer_json_has_seed_migration() {
    let path = showcase_path();
    let json = build_json(&path).expect("build_json should succeed on examples/showcase");

    // Version 3 migrations (20260301_*.sql) are pure INSERT files — is_seed must be true.
    assert!(
        json.contains("\"is_seed\": true"),
        "JSON should mark at least one migration as is_seed: true"
    );
}
