//! Integration tests for the BASIC interpreter
//!
//! Tests are auto-discovered from tests/basic_programs/*.bas files.
//! Any .bas file with a matching .expected file is automatically tested.
//! This ensures parity with the basic_interpreter test suite.

use emu6502::BasicHarness;
use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use tracing::info;

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .init();
    });
}

const INTERPRETER_BINARY: &str = "../build/original/basic.bin";
const TEST_DIR: &str = "../tests/basic_programs";

fn interpreter_exists() -> bool {
    PathBuf::from(INTERPRETER_BINARY).exists()
}

/// Discover all test names by scanning the filesystem for .bas + .expected pairs
/// Tests known to be incompatible with the original BASIC ROM.
///
/// These tests verify behaviour that the Rust interpreter enforces but the
/// original ROM handles differently due to its internal memory architecture:
///
/// - `error_string_too_long`: The Rust interpreter enforces a hard 255-byte
///    string length limit (LS error).  The ROM uses garbage-collected string
///    storage that shares the same ~1.5 KB pool as variables and arrays, so
///    the same concatenation typically hits an Out-of-Memory (OM) error before
///    the 255-byte limit is ever checked.
///
/// - `error_formula_too_complex`: The Rust interpreter counts temporary
///    string descriptors and raises ST after 15.  The ROM uses a 3-slot
///    descriptor stack but frees temporaries eagerly between sub-expressions,
///    so a long `A$+B$+C$+…` chain never actually fills the stack — each
///    intermediate result is freed before the next concatenation.
const EMU_EXCLUDED: &[&str] = &[
    "error_string_too_long",
    "error_formula_too_complex",
];

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
            if EMU_EXCLUDED.contains(&stem.as_str()) {
                return None;
            }
            let expected_path = dir.join(format!("{}.expected", stem));
            expected_path.exists().then_some(stem)
        })
        .collect();
    names.sort();
    names
}

fn read_optional_input_lines(base_path: &str) -> Vec<String> {
    let input_path = format!("{}.input", base_path);
    let Ok(raw) = fs::read_to_string(&input_path) else {
        return Vec::new();
    };

    raw.trim_end_matches(['\r', '\n'])
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect()
}

/// Run a single test case and compare output
fn run_and_compare(name: &str) {
    let base_path = format!("{}/{}", TEST_DIR, name);

    let program = fs::read_to_string(format!("{}.bas", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.bas: {}", name, e));

    let expected = fs::read_to_string(format!("{}.expected", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.expected: {}", name, e));

    let runtime_inputs = read_optional_input_lines(&base_path);

    if !interpreter_exists() {
        info!(program = name, "skipping: interpreter binary not built");
        return;
    }

    let output = BasicHarness::run_apple_ii_basic_with_inputs(
        INTERPRETER_BINARY,
        &program,
        &runtime_inputs,
        50_000_000,
    )
    .unwrap_or_else(|e| panic!("Failed to run {}: {}", name, e));

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
    init_tracing();

    let names = discover_test_names();
    assert!(!names.is_empty(), "No test programs found in {}", TEST_DIR);

    if !interpreter_exists() {
        info!(path = INTERPRETER_BINARY, "interpreter binary not found — skipping all");
        return;
    }

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

    eprintln!("All {}/{} emu6502 tests passed", names.len(), names.len());
}
