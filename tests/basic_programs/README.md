# BASIC Test Programs

This directory contains simple BASIC programs that will be used to test the interpreter once it's built.

Each `.bas` file represents a BASIC program with expected output documented in a corresponding `.expected` file.

## Test Program Naming Convention

- `*.bas` - BASIC source code
- `*.expected` - Expected output (one file per test)
- `*.txt` - Optional description/notes about the test

## Current Test Programs

These are minimal BASIC programs that test fundamental features:

1. `hello.bas` - Simple PRINT statement
2. `string.bas` - Multiple string PRINT statements
3. `arithmetic.bas` - Basic arithmetic operations
4. `variables.bas` - Variable assignment and usage
5. `expressions.bas` - Variable expressions and arithmetic
6. `for_loop.bas` - FOR/NEXT loop
7. `conditional.bas` - IF/THEN statement

## Running Tests

Once the interpreter binary is available, tests can be run using:
```bash
cargo test --workspace
```

Or run the test script directly:
```bash
bash scripts/run_interpreter_tests.sh
```
