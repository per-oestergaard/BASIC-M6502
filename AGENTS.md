
This is instructions for AI. The overall goal is to convert the BASIC interpreter into Rust.

The overall approach I want to take is -

1. Add a devcontainer that supports building the existing interpreter and also support the target Rust environment
2. Build the existing interpreter. Intel support is all I need.
3. Run or create relevant test on the built interpreter
4. Create Rust scaffolding for the interpreter
5. Chop up the work in 5 to 10 batches
6. For each batch do step 7 to 9
7. Create Rust tests
8. Run Rust tests to ensure all tests fails
9. Implement the code and ensure the tests succeeds
10. Run all test to ensure the Rust interpreter works

---

## Standing Rules for AI Agents

### Temporary files
- Always use `./temp/` (workspace-relative) for any scratch, diagnostic, or staging files.
- Never use `/tmp` or any absolute system temp path.

### Tracing (diagnosis)
- Use the `tracing` crate (`tracing = "0.1"`) for all diagnostic output in both the assembler (`assembler6502`) and the emulator (`emu6502`) crates.
- Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` to binaries that need to print traces.
- Initialise the subscriber in `main()` with `tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init()`.
- Emit `trace!` calls (target `emu6502::cpu`, `assembler6502::preprocess`, etc.) at every meaningful decision point.
- **Do NOT use grep/sed/awk/od on source or binary files for diagnosis.**. Read the files directly or run the code with `RUST_LOG=trace` (or a targeted filter) and read the trace output instead.
- **Do NOT use grep/sed when examining output from cargo**, just redirect the output to a file in ./temp and look at the file instead of grep/sed. Use `RUST_LOG=trace` (or a targeted filter)

### Source file grammar (m6502.asm — MACRO-10 dialect for 6502)

The source is MACRO-10 assembler syntax targeting a 6502. Understanding the grammar is essential before touching the preprocessor.

#### Line structure (precedence top to bottom)

```
[label[:]] [opcode/directive/macro [operand]] [;comment]
```

1. **Comment** — `;` to end of line. Everything after `;` is ignored. A line may be *only* a comment.
2. **Block comment** — `COMMENT <delim>` … `<delim>` (delimiter is the first non-space char after `COMMENT`). Everything inside is ignored.
3. **Label** — a symbol at the start of the line followed by `:`. A label with `::` is a global strong label. A label alone on a line (no opcode) is legal and simply defines the symbol at the current PC. Labels are case-insensitive.
4. **Opcode / directive / macro call** — the first token after the optional label (and its colon). Execution-altering tokens (real 6502 opcodes, pseudo-ops, macro names) go here.
5. **Operand** — everything after the opcode on the same line, up to `;`. May contain expressions, register suffixes, or angle-bracket-quoted sub-expressions. Some instruction has no operands.

#### Symbols and expressions

- **Symbol names**: start with a letter, `_`, `.`, or `$`; followed by letters, digits, `_`, `.`, `$`. Case-insensitive.
- **Numeric literals**:
  - Decimal (default radix): bare digits, e.g. `40`
  - Octal: `^O<digits>`, e.g. `^O177` → `0x7F`
  - Hex: `$<hexdigits>`, e.g. `$FF`
- **Expression operators**: `+`, `-`, `*`, `/`; `&` (bitwise AND); `!` (bitwise OR); `^` prefix for base conversion (`^O` octal).
- **Angle-bracket quoting / grouping**: `<…>` is a balanced bracket used for three overlapping purposes: (1) group a macro body or conditional body, (2) group a sub-expression in arithmetic (exactly like parentheses), (3) delimit a macro parameter reference inside a body. All three are parsed identically. There is **no special `<<…>>` form** and no separate "low byte operator" `<` in this source — `<<WD>&^O377>` is just nesting where inner `<WD>` substitutes the parameter and outer `<…>` groups the result. Low/high bytes are extracted arithmetically with `&^O377` (low byte) or `/^O400` (high byte), never by a special `<`/`>` prefix.
- **Implementation rule — use a state machine, not regex**: Because `<>` nesting is arbitrarily deep, regex cannot parse it correctly. The right implementation is a character-by-character walk with an integer depth counter (`depth += 1` on `<`, `depth -= 1` on `>`). When `depth` returns to zero you have the complete balanced span. Evaluate it recursively. Never use a regex loop as a substitute — it is O(n²) and fails on even two levels of nesting.

#### Equates

```
SYMBOL == value    ; strong equate (double-equals), value is an expression
SYMBOL =  value    ; soft equate (single-equals), same meaning in this dialect
SYMBOL: .res n     ; reserve n bytes at current PC, binding SYMBOL to that PC
```

Both forms define `SYMBOL` as a numeric constant available to the whole file. Equates do **not** produce any output bytes.

#### Conditionals

```
IFE expr,<body>    ; assemble body if expr == 0
IFN expr,<body>    ; assemble body if expr != 0
IFNDEF sym,<body>  ; assemble body if sym is not defined
ELSE               ; else-branch for the most recent IF
ENDIF              ; close IF block (used when body spans multiple lines)
IF1,<body>         ; first pass only (treat as always-true for single-pass preprocessor)
IF2,<body>         ; second pass only (treat as always-false / skip)
```

- The body between `<` and the matching `>` is assembled only when the condition is satisfied.
- Multi-line bodies: when `,<` is the last character of the IF line, the body runs until a line that is exactly `>` (alone, possibly with whitespace).
- Conditions nest; each `IFE`/`IFN` pushes a frame onto a stack. Inactive frames suppress all content including nested conditionals.
- **Precedence**: conditional blocks are evaluated first, before macro expansion or opcode parsing.

#### DEFINE (macros)

```
DEFINE name [(param, ...)],<
    body lines
    body lines
>
```

- The definition is **not** emitted — just stored.
- A macro is **invoked** by writing its name as the opcode, followed by the argument(s).
- Single-argument macros: `MACRO_NAME arg` — the argument ends at end-of-line or `;`.
- In the body, `WD` (or whatever the param name is) is substituted literally for the argument. `<WD>+1` means "arg concatenated with `+1`", i.e., if arg is `FOO` the result is `FOO+1`.
- `<<expr>>` inside a body denotes a computed sub-expression to be evaluated, e.g. `<<WD>&^O377>` = low byte of WD.
- Macro bodies can contain further macro calls, conditionals, and real 6502 opcodes — all are expanded recursively.
- **DEFINE bodies are not instructions**: the preprocessor must skip them entirely during line output.

#### Directives (pseudo-ops that are not macros)

| Directive | Meaning |
|---|---|
| `ORG addr` | Set current assembly address |
| `.res n` | Reserve n bytes (no init) |
| `.byte v[,v…]` | Emit literal bytes |
| `.word v[,v…]` | Emit 16-bit little-endian words |
| `TITLE text` | Listing title — ignored |
| `SUBTTL text` | Listing sub-title — ignored |
| `SEARCH lib` | Include library — ignored |
| `SALL` | Suppress macro expansion listing — ignored |
| `RADIX n` | Set default numeric radix — record but do not emit |
| `PAGE` | Listing page break — ignored |
| `PRINTX text` | Print message during assembly — **suppress from output** |
| `REPEAT n,<body>` | Repeat body n times inline |
| `IRPC sym,<chars>` | Repeat body once per character in `chars` |
| `COMMENT delim … delim` | Block comment — ignored |
| `ADR(sym)` | Emit `.word sym` |
| `DCI "text"` | Emit ASCII bytes, last byte has bit 7 set |
| `DC "text"` | Emit plain ASCII bytes |
| `XWD hi,lo` | Emit a 16-bit word from two halves |
| `EXP val` | Emit a byte/word value |
| `BLOCK n` | Reserve n bytes (alias for `.res`) |

#### ACRLF special case
`ACRLF` is a **DEFINE** (defined as two bytes `13`, `10`). When it appears as a standalone token on a line (e.g. `REDDY: .byte ACRLF` or just `ACRLF` alone) the preprocessor must emit `.byte $0D, $0A`.

#### `.byte mnemonic` pattern
Several places use `.byte RTS`, `.byte INY`, etc. These are **not** directives with a string argument — they are a way to embed single implied-addressing 6502 opcodes as raw data bytes in the middle of code that jumps over them (a zero-page read trick or fall-through trick). The preprocessor must substitute the correct opcode byte:

| `.byte X` | emit |
|---|---|
| `.byte RTS` | `$60` |
| `.byte INY` | `$C8` |
| `.byte DEX` | `$CA` |
| `.byte CLC` | `$18` |
| `.byte SEC` | `$38` |
| `.byte TYA` | `$98` |
| `.byte TSX` | `$BA` |
| `.byte TXA` | `$8A` |


The MACRO-10 source (`m6502.asm`) uses a number of shorthand pseudo-ops that the preprocessor must expand correctly:

| Source pseudo-op | Correct 6502 expansion |
|---|---|
| `LDADY addr` | `LDA (addr),Y` — **indirect indexed**, NOT `LDA addr,Y` |
| `STADY addr` | `STA (addr),Y` — indirect indexed |
| `LDADX addr` | `LDA (addr,X)` — indexed indirect |
| `STADX addr` | `STA (addr,X)` — indexed indirect |
| `LDAI n` | `LDA #n` |
| `LDXI n` | `LDX #n` |
| `LDYI n` | `LDY #n` |
| `CMPI n` | `CMP #n` |
| `CPXI n` | `CPX #n` |
| `CPYI n` | `CPY #n` |
| `ADCI n` | `ADC #n` |
| `SBCI n` | `SBC #n` |
| `ANDI n` | `AND #n` |
| `ORAI n` | `ORA #n` |
| `EORI n` | `EOR #n` |
| `STWD addr` | `STA addr` / `STY addr+1` |
| `STWX addr` | `STA addr` / `STX addr+1` |
| `STXY addr` | `STX addr` / `STY addr+1` |
| `LDWD addr` | `LDA addr` / `LDY addr+1` |
| `LDWX addr` | `LDA addr` / `LDX addr+1` |
| `LDXY addr` | `LDX addr` / `LDY addr+1` |
| `LDWDI <expr>` | `LDA #<expr` / `LDY #>expr` |
| `LDWXI <expr>` | `LDA #<expr` / `LDX #>expr` |
| `LDXYI <expr>` | `LDX #<expr` / `LDY #>expr` |
| `PSHWD addr` | `LDA addr` / `PHA` / `LDA addr+1` / `PHA` |
| `PULWD addr` | `PLA` / `STA addr+1` / `PLA` / `STA addr` |
| `CLR addr` | `LDA #0` / `STA addr` |
| `COM addr` | `LDA addr` / `EOR #$FF` / `STA addr` |
| `SYNCHK n` | `LDA #n` / `JSR SYNCHR` |
| `JEQ/JNE/JCS/JCC/JMI/JPL/JVS/JVC target` | inverted branch over JMP (long conditional jump) — see expansion rule below |
| `.byte INY` | opcode byte `$C8` (INY) — single implied instruction emitted as data byte |
| `.byte DEX` | opcode byte `$CA` |
| `.byte CLC` | opcode byte `$18` |
| `.byte SEC` | opcode byte `$38` |
| `.byte RTS` | opcode byte `$60` |
| `.byte TYA` | opcode byte `$98` |
| `.byte TSX` | opcode byte `$BA` |
| `ACRLF` | two bytes: `$0D`, `$0A` |

**Critical distinction**: `LDADY`/`STADY` use **indirect indexed** `(addr),Y` addressing (opcode $B1/$91), not absolute indexed `addr,Y` (opcode $B9/$99). The current preprocessor gets this wrong — it emits `LDA addr,Y` instead of `LDA (addr),Y`.

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

**Any plain `BEQ` / `BNE` / etc. that would exceed ±127 bytes must also be rewritten this way.** The assembler detects out-of-range relative branches in pass 2 and automatically rewrites them to the inverted-branch-over-JMP form. Do not emit a hard error for an out-of-range branch — rewrite it instead.

## Rust

- Remove unused variables. If they for some reason are needed, prefix them with _ to get rid of the warning.
- Format Rust code as normal
