# Testing Infrastructure

This document describes the testing infrastructure for the Microsoft BASIC interpreter for 6502.

## Overview

The testing infrastructure provides automated tests for the BASIC interpreter using a custom 6502 emulator harness. Tests are written in Rust and use the `BasicHarness` to execute BASIC programs and verify their output.

## Components

### 1. BasicHarness (`emu6502` crate)

The `BasicHarness` is a test harness that provides:
- Loading and execution of the BASIC interpreter binary
- Running BASIC programs
- Capturing output for verification
- Graceful handling of missing interpreter binary

**Location**: `emu6502/src/lib.rs`

**Key Methods**:
- `new()` - Create a new harness instance
- `load_basic_interpreter(path)` - Load the interpreter binary
- `execute_program(program)` - Execute a BASIC program
- `is_basic_loaded()` - Check if interpreter is loaded

### 2. Interpreter Tests

Automated test suite for the BASIC interpreter.

**Location**: `emu6502/tests/interpreter_tests.rs`

**Test Categories**:
- Basic functionality tests (print, variables, arithmetic)
- Program file tests (loads and runs .bas files)
- Graceful skipping when interpreter binary is unavailable

### 3. Test Programs

Collection of BASIC test programs covering various language features.

**Location**: `tests/basic_programs/`

**Programs**:
1. **string.bas** - String handling and variable assignments
2. **expressions.bas** - Arithmetic expressions and operations
3. **numeric_sequence.bas** - Numeric program sequencing and 2-stage modules
4. **variables.bas** - Variable assignment patterns
5. **loop.bas** - FOR loops and iteration
6. **arrays.bas** - Array operations
7. **functions.bas** - Built-in BASIC functions
8. **conditions.bas** - Conditional statements

### 4. Build Scripts

Scripts for building and testing the interpreter.

**Location**: `scripts/`

**Scripts**:
- `build_original.sh` - Builds the original interpreter binary
- `run_interpreter_tests.sh` - Runs all interpreter tests

## Building the Original Interpreter

The original BASIC interpreter must be built from the assembly source before running tests:

```bash
bash scripts/build_original.sh
```

**Requirements**:
- cc65 toolchain (ca65 assembler and ld65 linker)

**Output**:
- `build/original/basic.bin` - The assembled interpreter binary
- `build/original/basic.lst` - Assembly listing file
- `build/original/basic.o` - Object file

### Installing cc65

**Ubuntu/Debian**:
```bash
sudo apt-get install cc65
```

**macOS (Homebrew)**:
```bash
brew install cc65
```

## Running Tests

### Run All Tests

```bash
bash scripts/run_interpreter_tests.sh
```

Or use Cargo directly:

```bash
cargo test --workspace
```

### Run Specific Test Package

```bash
cargo test -p emu6502
```

### Run Specific Test

```bash
cargo test -p emu6502 --test interpreter_tests test_variable_assignment
```

### Run with Output

```bash
cargo test -p emu6502 --test interpreter_tests -- --nocapture
```

## Test Behavior

### Graceful Skipping

Tests are designed to skip gracefully when the interpreter binary is not available:

1. Tests check if `build/original/basic.bin` exists
2. If missing, tests print a warning and skip
3. Tests continue to run, allowing verification of test infrastructure
4. No test failures occur due to missing binary

This allows:
- Running tests in CI without the binary
- Local development without building the interpreter
- Incremental test development

### Test Output

Tests that require the interpreter will output:
```
Skipping test: BASIC interpreter not available
```

Tests will still verify:
- Test harness creation
- File loading logic
- Test program structure

## CI/CD Integration

The GitHub Actions workflow automatically:

1. Installs required tools (Rust, cc65)
2. Builds the original interpreter
3. Runs all tests
4. Reports results

**Workflow**: `.github/workflows/ci.yml`

**Triggers**:
- Push to main branch
- Push to copilot/** branches
- Pull requests to main

## Writing New Tests

### Adding a Unit Test

Add to `emu6502/tests/interpreter_tests.rs`:

```rust
#[test]
fn test_new_feature() {
    let mut harness = BasicHarness::new();
    harness.load_basic_interpreter(&get_basic_binary_path()).ok();
    
    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let program = r#"10 PRINT "TEST""#;
    let result = harness.execute_program(program);
    assert!(result.is_ok());
}
```

### Adding a Test Program

1. Create `tests/basic_programs/newtest.bas`
2. Add test in `interpreter_tests.rs`:

```rust
#[test]
fn test_newtest_program() {
    let program_path = get_test_program_path("newtest.bas");
    
    if !program_path.exists() {
        eprintln!("Skipping test: newtest.bas not found");
        return;
    }

    let mut harness = BasicHarness::new();
    harness.load_basic_interpreter(&get_basic_binary_path()).ok();
    
    if !harness.is_basic_loaded() {
        eprintln!("Skipping test: BASIC interpreter not available");
        return;
    }

    let result = run_test_program(&mut harness, &program_path);
    assert!(result.is_ok());
}
```

## Test Coverage

Current test coverage includes:

- ✅ String handling
- ✅ Variable assignments
- ✅ Arithmetic expressions
- ✅ Numeric program sequences
- ✅ 2-stage operational modules
- ✅ Loop constructs
- ✅ Array operations
- ✅ Built-in functions
- ✅ Conditional statements

## Troubleshooting

### Tests Skip with "BASIC interpreter not available"

**Solution**: Build the interpreter:
```bash
bash scripts/build_original.sh
```

### Build fails: "cc65 toolchain not found"

**Solution**: Install cc65:
- Ubuntu: `sudo apt-get install cc65`
- macOS: `brew install cc65`

### Tests fail with assembly errors

**Solution**: Check that `m6502.asm` is valid 6502 assembly code and compatible with cc65.

### Permission denied on scripts

**Solution**: Make scripts executable:
```bash
chmod +x scripts/*.sh
```

## Future Enhancements

Potential areas for expansion:

- Full 6502 instruction emulation
- Interactive debugging support
- Performance benchmarking
- Code coverage reporting
- Integration with hardware emulators
- More comprehensive BASIC language tests
