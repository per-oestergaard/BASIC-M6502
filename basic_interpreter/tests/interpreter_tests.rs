//! Integration tests for the Rust BASIC interpreter
//!
//! Tests are auto-discovered from tests/basic_programs/*.bas files.
//! Any .bas file with a matching .expected file is automatically tested.
//! This ensures parity with the emu6502 test suite — no test can be forgotten.

use std::fs;
use std::path::PathBuf;

const TEST_DIR: &str = "../tests/basic_programs";

/// Discover all test names by scanning the filesystem for .bas files
/// that have a corresponding .expected file.
fn discover_test_names() -> Vec<String> {
    let dir = PathBuf::from(TEST_DIR);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", dir.display(), e))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? != "bas" {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            // Only include if .expected file exists
            let expected_path = dir.join(format!("{}.expected", stem));
            expected_path.exists().then_some(stem)
        })
        .collect();
    names.sort();
    names
}

/// Run a single test case: load .bas, optional .input, compare against .expected
fn run_and_compare(name: &str) {
    let dir = PathBuf::from(TEST_DIR);

    let program = fs::read_to_string(dir.join(format!("{}.bas", name)))
        .unwrap_or_else(|e| panic!("Failed to read {}.bas: {}", name, e));

    let expected = fs::read_to_string(dir.join(format!("{}.expected", name)))
        .unwrap_or_else(|e| panic!("Failed to read {}.expected: {}", name, e));

    let input_path = dir.join(format!("{}.input", name));
    let inputs: Vec<String> = if input_path.exists() {
        fs::read_to_string(&input_path)
            .unwrap_or_else(|e| panic!("Failed to read {}.input: {}", name, e))
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let output = basic_interpreter::run_program(&program, &inputs)
        .unwrap_or_else(|e| format!("ERROR: {}", e));

    assert_eq!(
        output.trim(),
        expected.trim(),
        "Output mismatch for '{}'",
        name,
    );
}

/// Master test: discovers ALL .bas files with .expected counterparts
/// and runs each one. Collects all failures before reporting.
#[test]
fn all_basic_programs() {
    let names = discover_test_names();
    assert!(!names.is_empty(), "No test programs found in {}", TEST_DIR);

    let mut failures = Vec::new();

    for name in &names {
        let result = std::panic::catch_unwind(|| run_and_compare(name));
        if let Err(e) = result {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            failures.push((name.clone(), msg));
        }
    }

    if !failures.is_empty() {
        let report = failures
            .iter()
            .map(|(name, msg)| format!("  FAIL {}: {}", name, msg))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "\n{}/{} tests failed:\n{}\n",
            failures.len(),
            names.len(),
            report
        );
    }

    eprintln!("All {}/{} basic_interpreter tests passed", names.len(), names.len());
}
