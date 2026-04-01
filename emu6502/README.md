# emu6502

A lightweight 6502 CPU core plus a small harness layer used for reconstructing and validating a Microsoft BASIC binary during a binary-first fidelity workflow.

## Features
- Core 6502 (NMOS) instruction set (no undocumented opcodes yet)
- Decimal mode (approximate) for ADC/SBC
- IRQ & NMI handling with vector dispatch
- Cycle counting (base + branch taken; page-cross penalties TODO)
- Hookable memory-mapped I/O (read & write per address)
- Harness with queued keyboard input and captured output
- Convenience helpers: run until BRK, until predicate, until output suffix

## Planned / TODO
- Page-cross cycle penalties
- Complete undocumented opcodes (if required by target BASIC)
- More accurate BCD flag behavior nuances
- Memory layout abstraction and bank/IO segmentation

## Quick Start
Add this crate to a workspace (already done here). Example usage:

```rust
use emu6502::{Cpu, BasicHarness};

fn demo(){
    let mut h = BasicHarness::new();
    let program = [0xA9,b'A',0x8D,0x02,0x00,0x00]; // LDA #'A'; STA $0002; BRK
    h.load_and_run(0x0800,&program,10_000).unwrap();
    assert_eq!(h.output_string(), "A");
}
```

Run tests:
```
cargo test -p emu6502
```

Run one of the checked-in BASIC sample programs through the emulator harness:
```
cargo run -p emu6502 --example run_basic -- for_loop
```

## Integrating a BASIC Binary
1. Place original binary at `disasm/orig/basic.bin`.
2. Run `cargo test -p emu6502 -- --ignored` (future: we may mark long BASIC start tests ignored).
3. Use harness `load_binary_and_run` or build a richer environment mapping expected I/O vectors.

## Safety Notes
- Hook closures use raw pointers only where necessary; current design confines unsafe to a minimal area.
- Not constant-time; do not use for security-sensitive emulation.

## License
See repository root `LICENSE`.
