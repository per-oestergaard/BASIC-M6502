Assembly Adaptation Plan (Step 2)

Goal: Produce a byte-faithful Microsoft 6502 BASIC 8K (Apple target REALIO=4) binary from original source `m6502.asm` using ca65.

Immediate Error Causes (from first stub attempt):
1. Mixed legacy equality forms: lines with `SYMBOL==value` remain; ca65 expects either `SYMBOL = value` or `.set`. Some `==` still present inside conditional-bodies not stripped.
2. Octal constants (e.g. `^O40000`) not converted in fallback path because stub translator leaves untouched inside excluded / included conditional blocks.
3. Conditional directives (`IFE`, `IFN`) left literal (some included body still contains raw original lines with `==` redefinitions causing redefinition errors).
4. Macro placeholders (e.g. `RORSW==1`) remained unnormalized.

Strategy Incremental Passes:
Pass A (Sanitize Equates):
- Normalize all `NAME==value` and `NAME== value` to `NAME = value` *after* conditional inclusion resolution.
- Convert remaining `^O` octal sequences globally.

Pass B (Light Conditionals):
- Evaluate top-level `IFE sym` and `IFN sym` where expression is simple (symbol or simple arithmetic) using accumulated symbols.
- For unsupported expressions, include body but comment header as `;COND-UNRESOLVED`.

Pass C (Macros to Comments):
- Replace `DEFINE` macro bodies with commented placeholder and skip body lines until matching `>` (already partly done) ensuring no raw parameter artifacts remain (`<WD>` -> comment).

Pass D (Pseudo Opcodes LDAI etc.):
- Replace `LDAI expr` with `LDA #expr` and similarly for LDXI/LDYI/ADCI/SBCI/CMPI/EORI.

Pass E (Data Directives):
- Replace bare decimal on line with `.byte` or `.word` depending range.
- Replace `BLOCK n` with `.res n`.
- Replace `ADR(symbol)` with `.word symbol`.

Pass F (Validation Loop):
- Assemble; capture undefined symbols list. For each undefined symbol only ever used as operand (not assigned) ensure earlier conditional didn't strip its definition incorrectly.

Exit Criteria for Step 2:
1. Assembles with zero errors (may have undefined symbol warnings initially; drive to zero).
2. Emits binary of plausible size (<12 KB for 8K variant including tables) with vectors populated.
3. Emulator harness loads and reaches READY (heuristic: outputs prompt sequence) within cycle budget.
4. SHA256 of binary stored at `build/original/basic.bin.sha256` for future regression.

Next Implementation Actions:
1. Add new script `scripts/translate_min_ca65.py` focusing only on passes A-E in a linear single-pass + some regex rewrites (simpler than repairing v2 / structured).
2. Integrate it as first choice in `build_original.sh`.
3. Iterate until assembly succeeds.

Note: We do not attempt full macro semantic expansion (performance-focused macros). For fidelity we only need final bytes; macros expand to standard instructions already represented in source following DEFINE usage sites. If DEFINE-generated constructs are required (e.g., ROR emulation when RORSW=0) we will manually inject those fallback sequences.
