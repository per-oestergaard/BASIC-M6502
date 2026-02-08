# Microsoft BASIC for 6502 Microprocessor - Version 1.1

## Historical Significance

This assembly language source code represents one of the most historically significant pieces of software from the early personal computer era. It is the complete source code for **Microsoft BASIC Version 1.1 for the 6502 microprocessor**, originally developed and copyrighted by Microsoft in 1976-1978.

### Why This Document is Historically Important

#### 1. Foundation of the Personal Computer Revolution

- This BASIC interpreter was the software foundation that powered many of the most influential early personal computers
- It democratized programming by making it accessible to non-technical users through a simple, English-like programming language
- Without this software, the personal computer revolution might have developed very differently

#### 2. Microsoft's Early Success

- This represents some of Microsoft's earliest and most successful software
- The licensing of this BASIC interpreter to multiple computer manufacturers was crucial to Microsoft's early business model
- It established Microsoft as a dominant force in personal computer software before MS-DOS or Windows

#### 3. Multi-Platform Compatibility

- This single codebase was designed to run on multiple different computer systems of the era
- The conditional compilation system allowed the same source code to target different hardware platforms
- This approach influenced how software would be developed for decades to come

## Supported Computer Systems

The source code includes conditional compilation support for multiple pioneering computer systems:

- **Apple II** (`REALIO=4`) - Steve Jobs and Steve Wozniak's revolutionary home computer
- **Commodore PET** (`REALIO=3`) - One of the first complete personal computers
- **Ohio Scientific (OSI)** (`REALIO=2`) - Popular among hobbyists and schools
- **MOS Technology KIM-1** (`REALIO=1`) - An influential single-board computer
- **PDP-10 Simulation** (`REALIO=0`) - For development and testing purposes

## Technical Specifications

- **Language**: 6502 Assembly Language
- **Target Processor**: MOS Technology 6502 8-bit microprocessor
- **Memory Footprint**: 8KB ROM version
- **Features**: Complete BASIC interpreter with floating-point arithmetic
- **Architecture**: Designed for both ROM and RAM configurations

## Key Features

### Programming Language Support

- Full BASIC language implementation
- Floating-point arithmetic
- String handling and manipulation
- Array support (both integer and string arrays)
- Mathematical functions and operators
- Input/output operations

### Memory Management

- Efficient memory utilization for 8-bit systems
- String garbage collection
- Dynamic variable storage
- Stack-based expression evaluation

### Hardware Abstraction

- Configurable I/O routines for different computer systems
- Terminal width adaptation
- Character input/output abstraction
- Optional disk storage support

## Development History

The source code includes detailed revision history showing active development:

- **July 27, 1978**: Fixed critical bugs in FOR loop variable handling and statement parsing
- **July 1, 1978**: Memory optimization and garbage collection improvements
- **March 9, 1978**: Enhanced string function capabilities
- **February 25, 1978**: Input flag corrections and numeric precision improvements
- **February 11, 1978**: Reserved word parsing enhancements
- **January 24, 1978**: User-defined function improvements

## Cultural Impact

### Educational Influence

- This BASIC interpreter introduced millions of people to computer programming
- It was the first programming language for countless programmers who later became industry leaders
- The simple, interactive nature of BASIC made computers approachable for non-technical users

### Industry Standardization

- Microsoft's BASIC became the de facto standard for personal computer programming
- The design patterns and conventions established here influenced later programming languages and development tools
- The multi-platform approach pioneered techniques still used in modern software development

### Business Model Innovation

- The licensing of this software to multiple hardware manufacturers created Microsoft's early business model
- It demonstrated the viability of software as a standalone business, separate from hardware
- This approach became the template for the entire software industry

## Technical Innovation

### Compiler Technology

- Advanced macro system for code generation
- Sophisticated conditional compilation for multi-platform support
- Efficient symbol table management
- Optimized code generation for memory-constrained systems

### Runtime System

- Stack-based expression evaluator
- Dynamic memory management
- Real-time garbage collection
- Interactive command processing

## Legacy

This source code represents the foundation upon which the modern software industry was built. The techniques, patterns, and business models pioneered in this BASIC interpreter directly influenced:

- The development of MS-DOS and subsequent Microsoft operating systems
- The standardization of programming language implementations
- The establishment of software licensing as a business model
- The democratization of computer programming

## File Information

- **Filename**: `m6502.asm`
- **Lines of Code**: 6,955 lines
- **Copyright**: Microsoft Corporation, 1976-1978
- **Version**: 1.1
- **Assembly Format**: Compatible with period assemblers for 6502 development

---

*This document represents a crucial piece of computing history - the source code that helped launch the personal computer revolution and established Microsoft as a software industry leader.*

## Development Environment (Modern Port Effort)

This repository now includes a dev container to support:

- Original assembly exploration (with a planned translation pipeline to a modern assembler)
- Rust (stable) for the incremental re-implementation of the interpreter

### Quick Start (Dev Container)

1. Open in VS Code and select: Reopen in Container.
2. After build, verify toolchain via `scripts/postCreate.sh` (runs automatically).
3. Run `bash scripts/build_original.sh` (currently a stub; exits non-zero until translation implemented).

### Porting Roadmap (High-Level)

1. ✓ Provide dev container with cc65 and Rust toolchain.
2. Provide translation of legacy assembler macros to ca65.
3. Produce reference binary / behavioral tests (parsing, expression eval, PRINT, variables).
4. Scaffold Rust crate replicating memory model + tokenizer.
5. Add incremental test batches mapping original behavior to Rust.
6. Expand until full BASIC feature set covered.

See `docs/ASSEMBLY_PORTING_NOTES.md` for ongoing details.

## Testing

This repository includes multiple layers of testing:

### Step 1-2: Build Infrastructure Tests
- Run the build script: `bash scripts/build_original.sh`
- Currently expected to fail until assembly translation is complete

### Step 3: Interpreter Tests (Current Implementation)
Run interpreter tests using either:

```bash
# Run all tests including interpreter tests
cargo test --workspace

# Run only interpreter tests
bash scripts/run_interpreter_tests.sh

# Or directly
cargo test -p emu6502 --test interpreter_tests
```

**Note:** Interpreter tests will skip execution if the interpreter binary is not yet built. 
The test infrastructure is ready and will automatically run once `build/original/basic.bin` is available.

Test programs are located in `tests/basic_programs/` with the following structure:
- `*.bas` - BASIC source code
- `*.expected` - Expected output for each test

Current test programs:
- `hello.bas` - Simple PRINT statement
- `string.bas` - Multiple string PRINT statements
- `arithmetic.bas` - Basic arithmetic operations
- `variables.bas` - Variable assignment and usage
- `expressions.bas` - Variable expressions and arithmetic
- `for_loop.bas` - FOR/NEXT loop
- `conditional.bas` - IF/THEN statement

### Step 4+: Rust Implementation Tests
- Run unit tests: `cargo test --workspace`
- Tests for the 6502 emulator are in `emu6502/src/lib.rs`
- Tests for the BASIC harness are in `emu6502/src/harness.rs`
