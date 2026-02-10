use emu6502::BasicHarness;
use std::fs;
use std::path::PathBuf;

/// Helper function to get the path to the original BASIC binary
fn get_basic_binary_path() -> PathBuf {
    PathBuf::from("build/original/basic.bin")
}

/// Helper function to check if the original BASIC binary is available
fn is_basic_binary_available() -> bool {
    get_basic_binary_path().exists()
}

/// Helper function to get test program path
fn get_test_program_path(name: &str) -> PathBuf {
    PathBuf::from(format!("tests/basic_programs/{}", name))
}

/// Helper function to load and execute a test program
fn run_test_program(harness: &mut BasicHarness, program_path: &PathBuf) -> Result<String, String> {
    let program = fs::read_to_string(program_path)
        .map_err(|e| format!("Failed to read test program: {}", e))?;
    harness.execute_program(&program)
}

#[test]
fn test_harness_creation() {
    let harness = BasicHarness::new();
    assert!(!harness.is_basic_loaded());
}

#[test]
fn test_basic_binary_loading() {
    if !is_basic_binary_available() {
        eprintln!("Skipping test: build/original/basic.bin not available");
        eprintln!("Run 'bash scripts/build_original.sh' to build the original interpreter");
        return;
    }

    let mut harness = BasicHarness::new();
    let result = harness.load_basic_interpreter(&get_basic_binary_path());

    assert!(
        result.is_ok(),
        "Failed to load BASIC interpreter: {:?}",
        result
    );
    assert!(harness.is_basic_loaded());
}

#[test]
fn test_simple_print_statement() {
    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let program = r#"10 PRINT "HELLO WORLD""#;
    let result = harness.execute_program(program);

    assert!(result.is_ok());
    assert!(result.unwrap().contains("HELLO WORLD"));
}

#[test]
fn test_variable_assignment() {
    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let program = r#"10 LET A = 42
20 PRINT A"#;
    let result = harness.execute_program(program);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("42"));
}

#[test]
fn test_multiple_variable_assignments() {
    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let program = r#"10 LET X = 10
20 LET Y = 20
30 LET Z = 30
40 PRINT X
50 PRINT Y
60 PRINT Z"#;
    let result = harness.execute_program(program);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("10"));
    assert!(output.contains("20"));
    assert!(output.contains("30"));
}

#[test]
fn test_arithmetic_operations() {
    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let program = r#"10 LET A = 5
20 LET B = 3
30 PRINT A+B
40 PRINT A-B
50 PRINT A*B"#;
    let result = harness.execute_program(program);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("8")); // 5+3
    assert!(output.contains("2")); // 5-3
    assert!(output.contains("15")); // 5*3
}

#[test]
fn test_string_program() {
    let program_path = get_test_program_path("string.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: string.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(result.is_ok(), "String test program failed: {:?}", result);
}

#[test]
fn test_expressions_program() {
    let program_path = get_test_program_path("expressions.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: expressions.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(
        result.is_ok(),
        "Expressions test program failed: {:?}",
        result
    );
}

#[test]
fn test_numeric_sequence_program() {
    let program_path = get_test_program_path("numeric_sequence.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: numeric_sequence.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(
        result.is_ok(),
        "Numeric sequence test program failed: {:?}",
        result
    );
}

#[test]
fn test_variables_program() {
    let program_path = get_test_program_path("variables.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: variables.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(
        result.is_ok(),
        "Variables test program failed: {:?}",
        result
    );
}

#[test]
fn test_loop_program() {
    let program_path = get_test_program_path("loop.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: loop.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(result.is_ok(), "Loop test program failed: {:?}", result);
}

#[test]
fn test_arrays_program() {
    let program_path = get_test_program_path("arrays.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: arrays.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(result.is_ok(), "Arrays test program failed: {:?}", result);
}

#[test]
fn test_functions_program() {
    let program_path = get_test_program_path("functions.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: functions.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(
        result.is_ok(),
        "Functions test program failed: {:?}",
        result
    );
}

#[test]
fn test_conditions_program() {
    let program_path = get_test_program_path("conditions.bas");

    if !program_path.exists() {
        eprintln!("Skipping test: conditions.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness
        .load_basic_interpreter(&get_basic_binary_path())
        .ok();

    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(
        result.is_ok(),
        "Conditions test program failed: {:?}",
        result
    );
}
