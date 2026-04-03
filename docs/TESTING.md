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
- 40 passing tests in the normal lane
- 0 ignored known-failure coverage targets

Current direct command:
```bash
cargo test -p emu6502 --test interpreter_tests
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
- `def_fn.bas` - DEF FN user-defined function evaluation
- `string_builtins.bas` - ASC/CHR$/STR$/VAL/LEFT$/RIGHT$/MID$
- `advanced_math.bas` - SQR/COS/SIN/TAN/EXP/LOG/ATN built-ins
- `fibonacci_sequence.bas` - Iterative Fibonacci sequence generation
- `string_sort.bas` - Multi-pass bubble sort over a string array with DATA-fed input

### Control Flow
- `for_loop.bas` - FOR/NEXT loops
- `nested_for.bas` - Nested FOR/NEXT loops
- `step_loop.bas` - Descending FOR/NEXT with STEP
- `conditional.bas` - IF/THEN statements
- `relops.bas` - Additional relational operators (`<>`, `>=`, `<=`)
- `goto.bas` - GOTO branch control flow
- `go_to_alias.bas` - `GO TO` spelling variant
- `gosub.bas` - GOSUB/RETURN subroutines
- `if_gosub.bas` - IF/THEN combined with GOSUB
- `gosub_state.bas` - Variable mutation across subroutine calls
- `on_goto.bas` - `ON ... GOTO` dispatch
- `logic_ops.bas` - `NOT`, `AND`, and `OR`
- `rem_statement.bas` - `REM` statement handling
- `restore_smoke.bas` - Minimal `RESTORE` statement dispatch smoke test
- `restore_read.bas` - `RESTORE` rewinds `READ` back to the start of `DATA`
- `clear_state.bas` - `CLEAR` resets variable state inside a running program

### Interactive Input
- `input_sum.bas` - Minimal `INPUT` smoke test using checked-in stdin-style fixture lines
- `gcd_input.bas` - Interactive Euclidean GCD program using `INPUT`
- `get_char.bas` - Minimal `GET` smoke test using checked-in character input
- `get_loop.bas` - `GET` loop regression that consumes multiple sequential characters
- `interactive_stars.bas` - Input-driven star banner adapted from a common BASIC teaching example

### Internet-Inspired Examples
- `hello_repeat.bas` - Adapted from the fixed-count FOR/NEXT "Hello, World!" example shown in the Wikipedia BASIC article
- `stars_banner.bas` - Adapted from the Wikipedia BASIC star-printing example, simplified for non-interactive automated testing
- `interactive_stars.bas` - Adapted from the interactive star-printing example shown in the Wikipedia BASIC article and driven by a checked-in input fixture

## Token Coverage

Using the active Apple II token table from `m6502.asm` (that is, the `DCI"..."` entries that are enabled for the current `REALIO=4`, `EXTIO=0`, `DISKO=0`, `NULCMD=0`, `GETCMD=1` build):

- Statement and reserved-word coverage is 28 / 34 tokens = 82.4% if `RUN` is counted via the harness, which enters each program and executes it with a `RUN` command.
- Direct statement coverage from checked-in `.bas` fixtures alone is 27 / 34 tokens = 79.4%.
- Built-in function coverage is 17 / 23 functions = 73.9%.
- Built-in function coverage is 18 / 23 functions = 78.3%.

Currently covered statement/reserved-word tokens include:
- `END`, `FOR`, `NEXT`, `DATA`, `INPUT`, `DIM`, `READ`, `LET`, `GOTO`, `RUN`, `IF`, `RESTORE`, `GOSUB`, `RETURN`, `REM`, `ON`, `DEF`, `PRINT`, `CLEAR`, `GET`, `TO`, `FN`, `THEN`, `NOT`, `STEP`, `AND`, `OR`, `GO`

Currently covered built-in functions include:
- `SGN`, `INT`, `ABS`, `SQR`, `LOG`, `EXP`, `COS`, `SIN`, `TAN`, `ATN`, `LEN`, `STR$`, `VAL`, `ASC`, `CHR$`, `LEFT$`, `RIGHT$`, `MID$`

Not yet covered by the interpreter fixtures are statement/reserved-word tokens such as:
- `STOP`, `WAIT`, `POKE`, `CONT`, `LIST`, `NEW`

Not yet covered by the interpreter fixtures are built-in functions such as:
- `USR`, `FRE`, `POS`, `RND`, `PEEK`

## Future Enhancements

Useful next additions would be:
- Additional interactive input coverage (multi-prompt `GET`/`INPUT` flows)
- State-management commands (`CONT`, `NEW`)
- Memory and machine-facing functions (`PEEK`, `POKE`, `USR`, `FRE`, `POS`)
- Random/trig remainder (`RND`, `ATN`)

Known currently failing probe areas:
- None in the checked-in interpreter integration suite at the moment

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
