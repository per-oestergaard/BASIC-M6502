# Disassembly Workflow (Binary-First Round Trip)

This directory holds configuration and scripts for producing a *re-assemblable* ca65 source tree from a known-good Microsoft 6502 BASIC 8K binary. The goal is a byte‑for‑byte identical rebuild to establish a canonical, testable baseline before porting to Rust.

## Stages
1. Place original binary at `orig/basic.bin`.
2. Run `scripts/disasm_roundtrip.sh` to generate initial disassembly using `da65`.
3. Iterate: mark data/code regions in `basic.cfg` until `diff` reports zero mismatches.
4. Commit resulting `src/basic.asm` + config + verification script.

## Files
- `basic.cfg` : da65 configuration (segments, ranges, symbols).
- `orig/` : original binary (not versioned if license unclear; add to .gitignore).
- `work/` : generated intermediate outputs.
- `src/` : curated ca65 source you will maintain and test.
- `scripts/` : helper scripts (disassemble, assemble, diff, report).

## Success Criteria
- `make verify` (or script equivalent) ends with `BINARY OK (identical)`.
- Assembled size and CRC32/MD5 match original.

## Moving to Tests
Once stable: write functional tests that drive the interpreter via a minimal harness calling its entry points.
