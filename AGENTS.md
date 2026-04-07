
This is instructions for AI. The overall goal is to convert the BASIC interpreter into Rust.

The overall approach I want to take is -

1. Create m6502_grammar.bnf so handle all aspects of m6502.asm ✅
2. The `bnf` crate was evaluated but its `ParseTree<'gram>` borrows from both the
   `Grammar` and the input `&str` through the same lifetime, making it impossible
   to return an owned value from a function. The crate was removed. The BNF grammar
   (`m6502_grammar.bnf`) remains as the canonical specification for the hand-written
   parser in `assembler6502/src/parser.rs`. ✅
3. Create a 6502 emulator in Rust and run the assembled binary in it, checking the
   output against the original BASIC interpreter.

---

## Standing Rules for AI Agents

### Terminal

- never redict to /dev/null or similar, always redirect to a file in ./temp/ and read the file instead
- never use a terminal command that I cannot allow to run automatically

### Temporary files
- Always use `./temp/` (workspace-relative) for any scratch, diagnostic, or staging files.
- Never use `/tmp` or any absolute system temp path.

### Tracing (diagnosis)
- Use the `tracing` crate (`tracing = "0.1"`) for all diagnostic output in both the assembler (`assembler6502`) and the emulator (`emu6502`) crates.
- Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to binaries that need to print traces.
- Initialise the subscriber in `main()` with `tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init()`.
- Emit `trace!` calls (target `emu6502::cpu`, `assembler6502::preprocess`, etc.) at every meaningful decision point.
- Keep repo-wide standing rules like this in `AGENTS.md`; repo memory is agent-only scratch context and is not part of the checked-in repository.
- **Do NOT use grep/sed/awk/od on source or binary files for diagnosis.**. Read the files directly or run the code with `RUST_LOG=trace` (or a targeted filter) and read the trace output instead.
- **Do NOT use grep/sed when examining output from cargo**, just redirect the output to a file in ./temp and look at the file instead of grep/sed. Use `RUST_LOG=trace` (or a targeted filter)

### Source file grammar (m6502.asm — MACRO-10 dialect for 6502)

The source is MACRO-10 assembler syntax targeting a 6502. Understanding the grammar is essential before touching the preprocessor.


#### Long conditional jump expansion (JEQ / JNE / etc.)

`JEQ target` and its siblings are **not** real 6502 instructions — they are pseudo-ops that expand to an inverted short branch over a `JMP`. The expansion is:

| Pseudo-op | Expansion |
|---|---|
| `JEQ target` | `BNE .+5` / `JMP target` |
| `JNE target` | `BEQ .+5` / `JMP target` |
| `JCS target` | `BCC .+5` / `JMP target` |
| `JCC target` | `BCS .+5` / `JMP target` |
| `JMI target` | `BPL .+5` / `JMP target` |
| `JPL target` | `BMI .+5` / `JMP target` |
| `JVS target` | `BVC .+5` / `JMP target` |
| `JVC target` | `BVS .+5` / `JMP target` |

The `.+5` offset is always 3 bytes past the `BNE` instruction (2 bytes for `BNE $03`, then 3 bytes for `JMP abs`). In the emitted text this is written as a generated local label, e.g.:

```
 BNE __LJ0
 JMP target
__LJ0:
```

**Any plain `BEQ` / `BNE` / etc. that resolves out of ±127 bytes must be rewritten this way.** Never emit an out-of-range branch error — always silently rewrite to the inverted-short-branch-over-JMP form. This is simpler than range checking and eliminates an entire class of assembler errors.

---

## Rust Rules

- Remove unused variables. If they for some reason are needed, prefix them with _ to get rid of the warning.
- Format Rust code as normal
- Use rust edition 2024 and do not use features from rust edition before 2024 if a better alternative exists in 2024. For example, use `let` expressions instead of `if let` where possible, and use the new `match` ergonomics.
- Use English for all variable names, comments, and documentation.
- Avoid mutable variables where possible
- prefer functional style
- Never use unsafe rust, and never use external C libraries or bindings. The entire codebase should be pure safe Rust.
- never use grep/sed/awk/od on source or binary files for diagnosis. Read the files directly or run the code with RUST_LOG=trace (or a targeted filter) and read the trace output instead.
- cloning is fine when it makes the code simpler and more readable, so don't be afraid to clone when needed. The codebase is small enough that performance is not a concern at this stage, so prioritise readability and simplicity over micro-optimisations.

## Rust interpreter

As the emulator is finished, the next step is to implement the BASIC interpreter in Rust. The interpreter must read and interpret BASIC directly. E.g., it has not benefit from using the assembler nor the emulator.

The interpreter is its own crate. The success criteria is to run the same suite of BASIC test (in tests/basic_programs/) and get the same output as the original BASIC interpreter. It should also support an interactive mode and the same commands as emu6502/examples/interactive.rs.
