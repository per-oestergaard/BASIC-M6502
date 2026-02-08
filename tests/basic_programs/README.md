# BASIC Test Programs

This directory contains test programs for the Microsoft BASIC interpreter for 6502.

## Test Programs

The following test programs are included:

1. **string.bas** - Tests string handling and manipulation
   - String literals
   - String variable assignments
   - Multiple string variables

2. **expressions.bas** - Tests arithmetic expressions and operations
   - Simple arithmetic operations (+, -, *, /)
   - Variable assignments
   - Expressions with variables
   - Complex expressions with operator precedence
   - Variable reassignment

3. **numeric_sequence.bas** - Tests numeric program sequencing
   - Proper line number ordering
   - 2-stage operational module configuration
   - Sequential execution verification

4. **variables.bas** - Tests variable assignment patterns
   - Single variable assignments
   - Multiple assignments with different values
   - Variable reassignment
   - Using variables in expressions
   - Chained assignments

5. **loop.bas** - Tests FOR loops
   - Simple counting loops
   - Loops with STEP
   - Nested loops

6. **arrays.bas** - Tests array operations
   - Array declarations with DIM
   - Array element assignment
   - Array element access
   - Arrays in loops

7. **functions.bas** - Tests built-in BASIC functions
   - ABS (absolute value)
   - INT (integer conversion)
   - SQR (square root)
   - RND (random number)

8. **conditions.bas** - Tests conditional statements
   - IF-THEN statements
   - Comparison operators (>, <, =)
   - Conditions with variables

## Running Tests

To run all interpreter tests:

```bash
bash scripts/run_interpreter_tests.sh
```

Or run tests directly with Cargo:

```bash
cargo test -p emu6502 --test interpreter_tests
```

## Test Requirements

Tests require the original BASIC interpreter binary at `build/original/basic.bin`.
To build it:

```bash
bash scripts/build_original.sh
```

Tests will skip gracefully if the binary is not available.
