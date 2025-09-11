# 6502 Assembly Porting Notes

The source `m6502.asm` uses a macro/conditional dialect (e.g. TITLE, SEARCH, SALL, RADIX, IFN/IFE, IRPC, DEFINE) that resembles the PDP-10 MACRO assembler environment once used for cross-assembling Microsoft BASIC. Modern 6502 assemblers (ca65, vasm, xa, etc.) do not natively accept this syntax.

## Immediate Goals

1. Identify minimal subset of directives/macros needed to produce a binary for reference tests.
2. Write a preprocessing translator that converts legacy constructs to ca65-compatible output.
3. Produce a deterministic binary (or at least a symbol map) for behavioral comparison while implementing the Rust version.

## Key Constructs To Translate

- IFN / IFE conditionals -> `.if` / `.endif` in ca65 (after mapping symbol arithmetic and equality rules)
- DEFINE macro definitions -> `.macro` / `.endmacro` (parameter re-mapping required)
- IRPC (Iterate over Characters) -> expand inline during preprocessing stage
- RADIX 10 -> no-op (affects numeric parsing; will normalize all numeric literals to explicit base)
- OCTAL constants (e.g. `^O200`) -> convert to `$80` or `0o200`
- BLOCK n -> `.res n`
- ADR(symbol) pseudo -> resolve to <symbol (lo/hi) depending on usage; may need heuristic

## Proposed Translation Pipeline

```text
m6502.asm --(python preprocessor)--> translated-m6502.s --(ca65)--> basic.o --(ld65 cfg)--> basic.bin
```

## Open Questions

- Target memory map and load address per REALIO variant.
- Minimal feature subset needed for functional regression tests (initial focus: expression evaluation, line editing, PRINT / LET, simple FOR/NEXT).

## Next Steps

1. Enumerate all unique directives/macros (script to scan file).
2. Prototype translator for a small slice (e.g., page zero section) to validate feasibility.
3. Decide binary format artifact (raw binary vs. segment-labeled).

## Risks

- Time required to fully translate all conditional assembly branches may exceed usefulness; consider extracting semantic tests directly from documentation instead.
