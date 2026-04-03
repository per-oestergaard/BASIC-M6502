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

Additional coverage fixtures include:

- `advanced_math.bas` - SQR/COS/SIN/TAN/EXP/LOG built-ins
- `clear_state.bas` - `CLEAR` resets variable state inside a running program
- `def_fn.bas` - DEF FN user-defined function evaluation
- `fibonacci_sequence.bas` - Iterative Fibonacci sequence generation
- `get_char.bas` - `GET` reads a single character from the runtime input stream
- `go_to_alias.bas` - `GO TO` syntax variant
- `gcd_input.bas` - Interactive Euclidean GCD using `INPUT`
- `input_sum.bas` - Minimal `INPUT` smoke test with stdin fixture
- `get_char.bas` - `GET` reads a single character from the runtime input stream
- `get_loop.bas` - `GET` can read multiple sequential characters inside a loop
- `logic_ops.bas` - `NOT`, `AND`, and `OR`
- `on_goto.bas` - `ON ... GOTO` dispatch
- `restore_read.bas` - `RESTORE` rewinds `READ` back to the start of `DATA`
- `rem_statement.bas` - `REM` comment handling
- `restore_smoke.bas` - Minimal `RESTORE` statement dispatch smoke test
- `string_sort.bas` - Bubble sort over a string array loaded from `DATA`
- `string_builtins.bas` - ASC/CHR$/STR$/VAL/LEFT$/RIGHT$/MID$

Internet-inspired example fixtures:

- `hello_repeat.bas` - adapted from the fixed-count `Hello, World!` FOR/NEXT example shown in the Wikipedia BASIC article
- `interactive_stars.bas` - adapted from the interactive star-printing example shown in the Wikipedia BASIC article and driven by a checked-in stdin fixture
- `stars_banner.bas` - adapted from the star-printing example shown in the Wikipedia BASIC article, simplified to avoid interactive input in automated tests

Input-driven fixtures may include a matching `.input` file. Each line in that file is delivered as one line of terminal input after `RUN` begins.
For `GET`-driven fixtures, those checked-in input characters are also exposed through the Apple II character-input hook.

## Running Tests

Once the interpreter binary is available, tests can be run using:
```bash
cargo test --workspace
```

Or run the test script directly:
```bash
bash scripts/run_interpreter_tests.sh
```
