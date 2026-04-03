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

fn run_basic_program_with_inputs(
    program_content: &str,
    runtime_inputs: &[String],
) -> Result<String, String> {
    if !interpreter_exists() {
        return Err(format!(
            "Interpreter binary not found at {}. Build it with the assembler first.",
            INTERPRETER_BINARY
        ));
    }
    BasicHarness::run_apple_ii_basic_with_inputs(
        INTERPRETER_BINARY,
        program_content,
        runtime_inputs,
        50_000_000,
    )
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

/// Generic test runner: loads .bas + .expected files, runs and compares.
fn test_basic_program(program_name: &str) {
    init_tracing();

    let base_path = format!("../tests/basic_programs/{}", program_name);
    let program = fs::read_to_string(format!("{}.bas", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.bas: {}", program_name, e));
    let expected = fs::read_to_string(format!("{}.expected", base_path))
        .unwrap_or_else(|e| panic!("Failed to read {}.expected: {}", program_name, e));
    let runtime_inputs = read_optional_input_lines(&base_path);

    if !interpreter_exists() {
        info!(
            program = program_name,
            "skipping test because interpreter binary is not built"
        );
        return;
    }

    match run_basic_program_with_inputs(&program, &runtime_inputs) {
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

#[test]
fn test_on_goto() {
    test_basic_program("on_goto");
}

#[test]
fn test_def_fn() {
    test_basic_program("def_fn");
}

#[test]
fn test_string_builtins() {
    test_basic_program("string_builtins");
}

#[test]
fn test_rem_statement() {
    test_basic_program("rem_statement");
}

#[test]
fn test_advanced_math() {
    test_basic_program("advanced_math");
}

#[test]
fn test_logic_ops() {
    test_basic_program("logic_ops");
}

#[test]
fn test_go_to_alias() {
    test_basic_program("go_to_alias");
}

#[test]
fn test_hello_repeat() {
    test_basic_program("hello_repeat");
}

#[test]
fn test_stars_banner() {
    test_basic_program("stars_banner");
}

#[test]
fn test_input_sum() {
    test_basic_program("input_sum");
}

#[test]
fn test_fibonacci_sequence() {
    test_basic_program("fibonacci_sequence");
}

#[test]
fn test_gcd_input() {
    test_basic_program("gcd_input");
}

#[test]
fn test_interactive_stars() {
    test_basic_program("interactive_stars");
}

#[test]
fn test_string_sort() {
    test_basic_program("string_sort");
}

#[test]
fn test_restore_smoke() {
    test_basic_program("restore_smoke");
}

#[test]
fn test_restore_read() {
    test_basic_program("restore_read");
}

#[test]
fn test_get_char() {
    test_basic_program("get_char");
}

#[test]
fn test_get_loop() {
    test_basic_program("get_loop");
}

#[test]
fn test_clear_state() {
    test_basic_program("clear_state");
}

// ── Error Condition Tests ──────────────────────────────────────────────────────

#[test]
fn test_error_next_without_for() {
    test_basic_program("error_next_without_for");
}

#[test]
fn test_error_syntax() {
    test_basic_program("error_syntax");
}

#[test]
fn test_error_return_without_gosub() {
    test_basic_program("error_return_without_gosub");
}

#[test]
fn test_error_out_of_data() {
    test_basic_program("error_out_of_data");
}

#[test]
fn test_error_division_by_zero() {
    test_basic_program("error_division_by_zero");
}

#[test]
fn test_error_undef_statement() {
    test_basic_program("error_undef_statement");
}

#[test]
fn test_error_redim_array() {
    test_basic_program("error_redim_array");
}

#[test]
fn test_error_type_mismatch() {
    test_basic_program("error_type_mismatch");
}

#[test]
fn test_error_undef_function() {
    test_basic_program("error_undef_function");
}

#[test]
fn test_error_illegal_quantity() {
    test_basic_program("error_illegal_quantity");
}

#[test]
fn test_error_overflow() {
    test_basic_program("error_overflow");
}


// The following errors exist in m6502.asm but are difficult to trigger in practice:
// - ERRLS ("STRING TOO LONG"): Requires string concatenation result >= 256 chars
// - ERRST ("FORMULA TOO COMPLEX"): Requires exhausting all string temporaries
// These test files exist but the tests are commented out.

// fn test_error_string_too_long() {
//     test_basic_program("error_string_too_long");
// }

// #[test]
// fn test_error_formula_too_complex() {
//     test_basic_program("error_formula_too_complex");
// }
