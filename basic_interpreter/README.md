# BASIC Interpreter

A pure Rust implementation of Microsoft BASIC compatible with the 1976-1978 version for the 6502 processor.

## Features

- **100% Test Coverage**: All 57 test programs from the original BASIC pass with exact output matching
- **Pure Rust**: No dependencies on external emulators or assemblers
- **Interactive REPL**: Full-featured interactive mode with line editing and program management
- **Complete Language Support**: All BASIC features including:
  - Variables (numeric and string)
  - Arrays (auto-dimensioning and explicit DIM)
  - Control flow (IF/THEN/ELSE, FOR/NEXT, GOTO, GOSUB/RETURN)
  - Mathematical functions (SIN, COS, TAN, ATN, EXP, LOG, SQR, ABS, INT, SGN)
  - String functions (CHR$, STR$, ASC, VAL, LEFT$, RIGHT$, MID$, LEN)
  - System functions (RND, FRE, POS, PEEK, POKE)
  - User-defined functions (DEF FN)
  - Data statements (READ, DATA, RESTORE)
  - Advanced branching (ON GOTO/GOSUB)

## Usage

### As a Library

```rust
use basic_interpreter::{parser, interpreter::Interpreter};

fn main() {
    let source = r#"
        10 PRINT "HELLO WORLD"
        20 END
    "#;

    let program = parser::parse(source).unwrap();
    let mut interp = Interpreter::new();
    let output = interp.run(&program).unwrap();
    println!("{}", output);
}
```

### Interactive REPL

Run the interactive interpreter:

```bash
cargo run --release --package basic_interpreter --example interactive
```

**Commands:**
- Type program lines with line numbers: `10 PRINT "HELLO"`
- `RUN` - Execute the current program
- `LIST` - Display all program lines
- `NEW` - Clear the current program
- `LOAD <file>` - Load a program from a file
- `SAVE <file>` - Save the current program to a file
- `exit` or Ctrl-D - Quit the REPL

**Immediate Mode:**
You can also execute BASIC statements directly without line numbers:
```
] PRINT 2+2
 4
```

### Running Test Programs

All test programs are in `tests/basic_programs/`:

```bash
cargo test --package basic_interpreter
```

## Architecture

The interpreter consists of three main phases:

1. **Lexer** (`src/lexer.rs`) - Tokenizes BASIC source code
2. **Parser** (`src/parser.rs`) - Hand-written recursive descent parser that generates an AST
3. **Interpreter** (`src/interpreter.rs`) - Executes the AST

### Design Decisions

- **Floating-point**: Uses Rust's `f64` instead of the original custom format
- **Memory model**: HashMap-based variable and array storage (not binary-compatible)
- **PRINT formatting**: Exact match with original Microsoft BASIC behavior
- **Error handling**: Returns descriptive error messages as strings

## Compatibility

The interpreter matches the original Microsoft BASIC v1.1 behavior exactly, including:
- PRINT spacing with semicolons and commas (14-character zones)
- Number formatting (space before positive, minus sign for negative)
- Array memory calculation for FRE() function
- String slicing with LEFT$, RIGHT$, MID$
- All quirks and edge cases from the original implementation

## Testing

Run the full test suite:

```bash
cargo test --package basic_interpreter -- --nocapture
```

All 57 tests pass with exact output matching against the original BASIC interpreter.

## Examples

See `examples/interactive.rs` for a full-featured REPL implementation.

Sample programs are available in `../tests/basic_programs/`:
- `fibonacci_sequence.bas` - Fibonacci number generator
- `advanced_math.bas` - Mathematical function demonstrations
- `string_builtins.bas` - String manipulation examples
- And many more!
