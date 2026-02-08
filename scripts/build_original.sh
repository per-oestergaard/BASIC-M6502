#!/bin/bash
# Build the original Microsoft BASIC interpreter for 6502
# Requires cc65 toolchain to be installed

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build/original"
SOURCE_FILE="$PROJECT_ROOT/m6502.asm"

echo "Building original BASIC interpreter..."
echo "======================================="

# Check if cc65 is installed
if ! command -v ca65 &> /dev/null; then
    echo "Error: cc65 toolchain not found"
    echo "Please install cc65 from: https://cc65.github.io/"
    echo ""
    echo "On Ubuntu/Debian:"
    echo "  sudo apt-get install cc65"
    echo ""
    echo "On macOS with Homebrew:"
    echo "  brew install cc65"
    exit 1
fi

# Create build directory
mkdir -p "$BUILD_DIR"

# Check if source file exists
if [ ! -f "$SOURCE_FILE" ]; then
    echo "Error: Source file not found: $SOURCE_FILE"
    exit 1
fi

echo "Source file: $SOURCE_FILE"
echo "Build directory: $BUILD_DIR"
echo ""

# Assemble the source file
echo "Assembling m6502.asm..."
cd "$BUILD_DIR"
ca65 -l basic.lst -o basic.o "$SOURCE_FILE" 2>&1 || {
    echo "Error: Assembly failed"
    exit 1
}

echo "Assembly successful!"
echo ""

# Link the object file
echo "Linking..."
ld65 -t none -o basic.bin basic.o 2>&1 || {
    echo "Error: Linking failed"
    exit 1
}

echo "Linking successful!"
echo ""

# Check output
if [ -f basic.bin ]; then
    SIZE=$(stat -f%z basic.bin 2>/dev/null || stat -c%s basic.bin 2>/dev/null)
    echo "Build complete!"
    echo "Output: $BUILD_DIR/basic.bin"
    echo "Size: $SIZE bytes"
    echo ""
    echo "You can now run the interpreter tests:"
    echo "  bash scripts/run_interpreter_tests.sh"
else
    echo "Error: Build failed - basic.bin not created"
    exit 1
fi
