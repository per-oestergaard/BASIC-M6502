# Microsoft BASIC for 6502 - Rust Implementation

This repository contains a **pure Rust reimplementation** of Microsoft BASIC v1.1 for the 6502 microprocessor, originally developed by Microsoft in 1976-1978.

## 🚀 Quick Start

**Run the interactive BASIC interpreter:**
```bash
cargo run --release --package basic_interpreter --example interactive
```

**Example session:**
```
] 10 PRINT "HELLO, WORLD!"
] 20 FOR I=1 TO 5
] 30 PRINT I;
] 40 NEXT I
] 50 END
] RUN
HELLO, WORLD!
 1  2  3  4  5
]
```

## Features

- ✅ **100% Test Coverage**: All 57 original test programs pass with exact output matching
- ✅ **Pure Rust**: Standalone interpreter with no external dependencies
- ✅ **Interactive REPL**: Full-featured command-line interface
- ✅ **Complete BASIC Implementation**: All language features from the original

## Quick Commands

### REPL Commands
- `RUN` - Execute the current program
- `LIST` - Display all program lines
- `NEW` - Clear the current program
- `LOAD <file>` - Load a program from file
- `SAVE <file>` - Save program to file
- Type statements without line numbers for immediate execution: `PRINT 2+2`

### Testing
```bash
# Run all tests
cargo test --workspace

# Run interpreter tests only
cargo test --package basic_interpreter

# Run with output
cargo test --package basic_interpreter -- --nocapture
```

## Language Support

### Complete BASIC Feature Set
- **Variables**: Numeric (A, X1, COUNT) and string (A$, NAME$)
- **Arrays**: Auto-dimensioning and explicit DIM
- **Control Flow**: IF/THEN/ELSE, FOR/NEXT, GOTO, GOSUB/RETURN, ON GOTO/GOSUB
- **Math Functions**: SIN, COS, TAN, ATN, EXP, LOG, SQR, ABS, INT, SGN, RND
- **String Functions**: CHR$, STR$, ASC, VAL, LEFT$, RIGHT$, MID$, LEN
- **System Functions**: FRE, POS, PEEK, POKE
- **User Functions**: DEF FN
- **Data**: READ, DATA, RESTORE
- **I/O**: PRINT, INPUT

### Example Programs

See `tests/basic_programs/` for 57 working examples:
- `fibonacci_sequence.bas` - Fibonacci numbers
- `advanced_math.bas` - Mathematical functions
- `string_builtins.bas` - String manipulation
- And many more!

## Repository Structure

```
basic_interpreter/     Rust BASIC interpreter (standalone)
├── src/
│   ├── lexer.rs      Tokenizer
│   ├── parser.rs     Hand-written recursive descent parser
│   └── interpreter.rs AST executor
├── examples/
│   └── interactive.rs Interactive REPL
└── tests/
    └── interpreter_tests.rs Test suite

emu6502/              6502 emulator (runs original binary)
assembler6502/        6502 assembler
tests/basic_programs/ Test programs with expected output
```

## Documentation

- [basic_interpreter/README.md](basic_interpreter/README.md) - Detailed interpreter documentation
- [ORIGINAL_README.md](ORIGINAL_README.md) - Historical context and original project information
- [emu6502/README.md](emu6502/README.md) - 6502 emulator documentation
- [AGENTS.md](AGENTS.md) - AI agent instructions and development notes

## Historical Context

This implementation is based on the original Microsoft BASIC source code for the 6502 processor. For the complete historical background, supported computer systems (Apple II, Commodore PET, OSI, KIM-1), and technical specifications, see [ORIGINAL_README.md](ORIGINAL_README.md).

## Architecture

The Rust interpreter uses a three-phase pipeline:

1. **Lexer** → Tokenizes BASIC source code
2. **Parser** → Generates Abstract Syntax Tree (AST)
3. **Interpreter** → Executes AST with HashMap-based variable storage

This design is completely independent of the 6502 assembly implementation and can be embedded in other Rust applications.

## Development

### Build
```bash
cargo build --release --package basic_interpreter
```

### Run Tests
```bash
cargo test --workspace
```

### Format Code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## License

See [LICENSE](LICENSE) for details.

## Contributing

This is a Microsoft repository. For contribution guidelines, please see [ORIGINAL_README.md](ORIGINAL_README.md).
