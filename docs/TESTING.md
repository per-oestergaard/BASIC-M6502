# Testing the BASIC Interpreter

This document describes the testing infrastructure for step 3 of the porting process: testing the current (non-Rust) Microsoft BASIC interpreter.

## Quick Start

Run all tests:
```bash
cargo test --workspace
```

Run only interpreter tests:
```bash
bash scripts/run_interpreter_tests.sh
```

**Note:** Tests will skip gracefully if the interpreter binary is not yet built. The test infrastructure is ready and waiting for the interpreter to be available at `build/original/basic.bin`.

## Overview

The test infrastructure is designed to:
1. Validate basic BASIC interpreter functionality
2. Work with the 6502 emulator harness
3. Support running once the interpreter binary is built
4. Provide clear examples for adding more tests

## Test Structure

### Test Programs (`tests/basic_programs/`)

Test programs consist of:
- `*.bas` files containing BASIC source code
- `*.expected` files containing the expected output

Example:
```
tests/basic_programs/
├── hello.bas          # 10 PRINT "HELLO WORLD"
└── hello.expected     # HELLO WORLD
```

### Integration Tests (`emu6502/tests/interpreter_tests.rs`)

The integration test suite:
1. Checks if the interpreter binary exists at `build/original/basic.bin`
2. If available, loads it into the 6502 emulator
3. Runs each test program
4. Compares actual output with expected output
5. If not available, tests are skipped with informative messages

## Running Tests

### Run All Tests
```bash
cargo test --workspace
```

### Run Only Interpreter Tests
```bash
# Using the convenience script
bash scripts/run_interpreter_tests.sh

# Or directly
cargo test -p emu6502 --test interpreter_tests
```

### Run with Verbose Output
```bash
cargo test -p emu6502 --test interpreter_tests -- --nocapture
```

## Current Test Status

With `build/original/basic.bin` present, the interpreter integration suite currently has:
- 20 passing tests in the normal lane
- 1 ignored known-failure coverage target

Current direct command:
```bash
cargo test -p emu6502 --test interpreter_tests
```

Run only the known-failure coverage targets:
```bash
cargo test -p emu6502 --test interpreter_tests -- --ignored
```

## Adding New Tests

To add a new test:

1. Create a BASIC program file in `tests/basic_programs/`:
   ```basic
   # tests/basic_programs/my_test.bas
   10 PRINT "TEST"
   20 END
   ```

2. Create the expected output file:
   ```
   # tests/basic_programs/my_test.expected
   TEST
   ```

3. Add a test function in `emu6502/tests/interpreter_tests.rs`:
   ```rust
   #[test]
   fn test_my_test() {
       test_basic_program("my_test");
   }
   ```

4. Run the tests:
   ```bash
   cargo test -p emu6502 --test interpreter_tests
   ```

## Test Categories

Current tests cover:

### Basic Output
- `hello.bas` - Simple PRINT statement
- `string.bas` - Multiple string PRINT statements

### Arithmetic
- `arithmetic.bas` - Addition, subtraction, multiplication, division
- `colon.bas` - Multiple statements on one line

### Variables
- `variables.bas` - Variable assignment and use
- `expressions.bas` - Variable expressions and arithmetic
- `arrays.bas` - DIM and indexed array assignment/access
- `array_bounds_error.bas` - BAD SUBSCRIPT detection for out-of-range array access

### Control Flow
- `for_loop.bas` - FOR/NEXT loops
- `nested_for.bas` - Nested FOR/NEXT loops
- `step_loop.bas` - Descending FOR/NEXT with STEP
- `conditional.bas` - IF/THEN statements
- `relops.bas` - Additional relational operators (`<>`, `>=`, `<=`)
- `goto.bas` - GOTO branch control flow
- `gosub.bas` - GOSUB/RETURN subroutines
- `if_gosub.bas` - IF/THEN combined with GOSUB
- `gosub_state.bas` - Variable mutation across subroutine calls

## Future Enhancements

Useful next additions would be:
- String operations
- More complex expressions
- Error handling
- DATA/READ/RESTORE
- Mathematical functions (SIN, COS, etc.)

Known currently failing probe areas:
- `DATA/READ` currently does not execute correctly in the Rust-emulated interpreter test path
- Some numeric and string built-ins currently hit an unimplemented opcode path in the CPU core during probing

Checked-in known-failure coverage target:
- `data_read.bas` - DATA/READ sequencing

## Integration with Build System

The test infrastructure integrates with:
1. **Cargo** - Standard Rust test framework
2. **Build scripts** - `scripts/build_original.sh` for building the interpreter
3. **CI/CD** - GitHub Actions workflow (`.github/workflows/ci.yml`) automatically runs tests on push and pull requests

### Continuous Integration

The CI workflow automatically:
- Builds the project with `cargo build --workspace`
- Runs all unit tests with `cargo test --workspace --lib`
- Runs interpreter integration tests with `cargo test -p emu6502 --test interpreter_tests`
- Reports test status (tests skip gracefully if interpreter binary is not available)

The workflow runs on:
- Push to `main`, `convert-to-rust`, or `copilot/**` branches
- Pull requests to `main` or `convert-to-rust` branches

## Troubleshooting

### Tests are being skipped
**Cause:** Interpreter binary not found
**Solution:** Complete step 2 (assembly translation and build)

### Tests fail with "I/O error"
**Cause:** Binary file exists but can't be read
**Solution:** Check file permissions on `build/original/basic.bin`

### Tests fail with "Execution error"
**Cause:** Interpreter loaded but crashed/timed out
**Solution:** Check cycle budget, verify binary is correct format

## Design Notes

The test harness uses `emu6502::BasicHarness` which:
- Provides a 6502 CPU emulator
- Supports memory-mapped I/O hooks for output capture
- Can load binary images at specified addresses
- Has configurable cycle budgets to prevent infinite loops
- Allows queuing input for interactive programs (future enhancement)

This approach allows testing the actual 6502 machine code without requiring real hardware.
