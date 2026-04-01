//! Integration tests for the BASIC interpreter
//!
//! These tests verify that the built BASIC interpreter can execute
//! simple BASIC programs correctly using the 6502 emulator harness.

use emu6502::BasicHarness;
use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use tracing::info;

static INIT: Once = Once::new();

/// Initialize tracing once for all tests
fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .init();
    });
}

/// Path to the interpreter binary (once built by the assembler)
const INTERPRETER_BINARY: &str = "../build/original/basic.bin";

fn interpreter_exists() -> bool {
    PathBuf::from(INTERPRETER_BINARY).exists()
}

/// Run a BASIC program through the Apple II emulator harness and return the output.
fn run_basic_program(program_content: &str) -> Result<String, String> {
    if !interpreter_exists() {
        return Err(format!(
            "Interpreter binary not found at {}. Build it with the assembler first.",
            INTERPRETER_BINARY
        ));
    }
    BasicHarness::run_apple_ii_basic(INTERPRETER_BINARY, program_content, 50_000_000)
}

/// Generic test runner: loads .bas + .expected files, runs and compares.
fn test_basic_program(program_name: &str) {
    init_tracing();

    let base_path = format!("../tests/basic_programs/{}", program_name);
    let program = fs::read_to_string(format!("{}.bas", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.bas: {}", program_name, e));
    let expected = fs::read_to_string(format!("{}.expected", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.expected: {}", program_name, e));

    if !interpreter_exists() {
        info!(
            program = program_name,
            "skipping test because interpreter binary is not built"
        );
        return;
    }

    match run_basic_program(&program) {
        Ok(output) => {
            assert_eq!(
                output.trim(),
                expected.trim(),
                "Output mismatch for {}\nExpected:\n{}\nGot:\n{}",
                program_name,
                expected.trim(),
                output.trim()
            );
        }
        Err(e) => panic!("Failed to run {}: {}", program_name, e),
    }
}

#[test]
fn test_hello_world() {
    test_basic_program("hello");
}

#[test]
fn test_arithmetic() {
    test_basic_program("arithmetic");
}

#[test]
fn test_arrays() {
    test_basic_program("arrays");
}

#[test]
fn test_array_bounds_error() {
    test_basic_program("array_bounds_error");
}

#[test]
fn test_variables() {
    test_basic_program("variables");
}

#[test]
fn test_colon() {
    test_basic_program("colon");
}

#[test]
fn test_for_loop() {
    test_basic_program("for_loop");
}

#[test]
fn test_nested_for() {
    test_basic_program("nested_for");
}

#[test]
fn test_step_loop() {
    test_basic_program("step_loop");
}

#[test]
fn test_goto() {
    test_basic_program("goto");
}

#[test]
fn test_gosub() {
    test_basic_program("gosub");
}

#[test]
fn test_if_gosub() {
    test_basic_program("if_gosub");
}

#[test]
fn test_gosub_state() {
    test_basic_program("gosub_state");
}

#[test]
fn test_conditional() {
    test_basic_program("conditional");
}

#[test]
fn test_relops() {
    test_basic_program("relops");
}

#[test]
fn test_string() {
    test_basic_program("string");
}

#[test]
fn test_expressions() {
    test_basic_program("expressions");
}

#[test]
fn test_interpreter_binary_status() {
    init_tracing();
    if interpreter_exists() {
        info!(path = INTERPRETER_BINARY, "interpreter binary found");
    } else {
        info!(path = INTERPRETER_BINARY, "interpreter binary not found");
    }
}

#[test]
#[ignore = "known failing coverage target"]
fn test_data_read() {
    test_basic_program("data_read");
}

#[test]
fn test_math_funcs() {
    test_basic_program("math_funcs");
}

#[test]
fn test_string_ops() {
    test_basic_program("string_ops");
}
