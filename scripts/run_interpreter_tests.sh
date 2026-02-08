#!/usr/bin/env bash
set -euo pipefail

## Purpose: Run interpreter tests for step 3 (current interpreter validation)
## This script runs the test suite that validates the BASIC interpreter
## functionality once the binary is available.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[test] Running interpreter integration tests..." >&2
echo "" >&2

# Check if interpreter binary exists
INTERPRETER_BINARY="$ROOT_DIR/build/original/basic.bin"
if [[ -f "$INTERPRETER_BINARY" ]]; then
    echo "✓ Interpreter binary found at: $INTERPRETER_BINARY" >&2
    echo "" >&2
else
    echo "✗ Interpreter binary not found at: $INTERPRETER_BINARY" >&2
    echo "  Note: This is expected until the assembly translation is complete." >&2
    echo "  The tests will be skipped but their structure is ready." >&2
    echo "" >&2
fi

# Run cargo test for the integration tests
echo "[test] Running cargo test for interpreter tests..." >&2
cargo test -p emu6502 --test interpreter_tests -- --nocapture

echo "" >&2
echo "[test] Integration tests complete!" >&2
