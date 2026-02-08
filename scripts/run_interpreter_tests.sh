#!/bin/bash
# Run interpreter tests for the BASIC interpreter

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BASIC_BIN="$PROJECT_ROOT/build/original/basic.bin"

echo "Running BASIC Interpreter Tests"
echo "================================"
echo ""

# Check if the BASIC binary exists
if [ ! -f "$BASIC_BIN" ]; then
    echo "Warning: BASIC interpreter binary not found at: $BASIC_BIN"
    echo "Tests will run but skip tests requiring the original interpreter."
    echo ""
    echo "To build the original interpreter, run:"
    echo "  bash scripts/build_original.sh"
    echo ""
fi

# Change to project root
cd "$PROJECT_ROOT"

# Run the tests
echo "Running tests with cargo..."
echo ""
cargo test -p emu6502 --test interpreter_tests -- --nocapture

echo ""
echo "Tests complete!"
