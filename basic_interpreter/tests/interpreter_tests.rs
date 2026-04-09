//! Integration tests for the Rust BASIC interpreter
//!
//! These tests run BASIC programs from tests/basic_programs/ and compare
//! the output with the expected output from .expected files.

use std::fs;
use std::path::PathBuf;

/// Read a BASIC program from file
fn read_program(name: &str) -> String {
    let path = PathBuf::from("../tests/basic_programs").join(format!("{}.bas", name));
    fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()))
}

/// Read expected output from file
fn read_expected(name: &str) -> String {
    let path = PathBuf::from("../tests/basic_programs").join(format!("{}.expected", name));
    fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()))
}

/// Read optional input lines for programs that use INPUT statement
fn read_input(name: &str) -> Vec<String> {
    let path = PathBuf::from("../tests/basic_programs").join(format!("{}.input", name));
    if path.exists() {
        fs::read_to_string(&path)
            .expect(&format!("Failed to read {}", path.display()))
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// Run a BASIC program and return its output
fn run_basic_program(name: &str) -> String {
    let program = read_program(name);
    let inputs = read_input(name);

    basic_interpreter::run_program(&program, &inputs).unwrap_or_else(|e| format!("ERROR: {}", e))
}

/// Test helper macro to generate test functions
macro_rules! basic_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let output = run_basic_program(stringify!($name));
            let expected = read_expected(stringify!($name));
            assert_eq!(
                output.trim(),
                expected.trim(),
                "Output mismatch for {}",
                stringify!($name)
            );
        }
    };
}

// Generate tests for all basic programs (non-error cases)
basic_test!(abs_int);
basic_test!(advanced_math);
basic_test!(arithmetic);
basic_test!(arrays);
basic_test!(asc_chr);
basic_test!(clear_state);
basic_test!(colon);
basic_test!(conditional);
basic_test!(data_read);
basic_test!(def_fn);
basic_test!(expressions);
basic_test!(fibonacci_sequence);
basic_test!(for_loop);
basic_test!(fre_function);
basic_test!(gcd_input);
basic_test!(gosub);
basic_test!(gosub_state);
basic_test!(goto);
basic_test!(go_to_alias);
basic_test!(hello);
basic_test!(hello_repeat);
basic_test!(if_gosub);
basic_test!(if_then_goto);
basic_test!(implicit_let);
basic_test!(input_sum);
basic_test!(len_function);
basic_test!(logic_ops);
basic_test!(math_funcs);
basic_test!(multiple_arrays);
basic_test!(multiple_statements);
basic_test!(negative_step);
basic_test!(nested_for);
basic_test!(nested_for_print);
basic_test!(nested_gosub);
basic_test!(nested_if);
basic_test!(new_command);
basic_test!(on_goto);
basic_test!(peek_poke);
basic_test!(pos_function);
basic_test!(print_comma);
basic_test!(print_semicolon);
basic_test!(relops);
basic_test!(rem_statement);
basic_test!(restore_data);
basic_test!(restore_read);
basic_test!(restore_smoke);
basic_test!(sgn_function);
basic_test!(spc_function);
basic_test!(step_loop);
basic_test!(stop_statement);
basic_test!(string);
basic_test!(string_builtins);
basic_test!(string_ops);
basic_test!(string_slicing);
basic_test!(tab_function);
basic_test!(val_str);
basic_test!(variables);
