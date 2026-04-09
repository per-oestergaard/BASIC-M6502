#!/bin/bash
# Start the original BASIC interpreter via the 6502 emulator (interactive REPL)
exec cargo run -p emu6502 --example interactive -- "$@"
